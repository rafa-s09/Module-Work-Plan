"""The speaking trumpet: passes orders to the domain and shouts the answers back. Decides nothing."""

from fastapi import APIRouter, Depends, Header
from pydantic import BaseModel

from app import auth, crew, db, hold, loot, voyage
from app.errors import ShipError

router = APIRouter(prefix="/api")


def get_conn():
    conn = db.connect()
    try:
        yield conn
    finally:
        conn.close()


def get_token(authorization: str | None = Header(None)) -> str:
    scheme, _, token = (authorization or "").partition(" ")
    if scheme != "Bearer" or not token:
        raise ShipError("not_logged_in")
    return token


def get_member(token: str = Depends(get_token), conn=Depends(get_conn)):
    return auth.current_member(conn, token)


def money(name: str, amount: int) -> dict:
    return {f"{name}_reales": amount, f"{name}_display": loot.format_reales(amount)}


def member_json(row) -> dict:
    return {
        "id": row["id"],
        "name": row["name"],
        "rank": row["rank"],
        "status": row["status"],
        "signed_at": row["signed_at"],
        "shares_display": crew.shares_display(crew.quarter_shares(row["rank"])),
    }


def me_json(member) -> dict:
    return member_json(member) | {"orders": auth.allowed_orders(member["rank"])}


# --- auth ---


class Login(BaseModel):
    username: str
    password: str


class NewAccount(BaseModel):
    crew_member_id: int
    username: str
    password: str


class OwnPassword(BaseModel):
    current_password: str
    new_password: str


class ResetPassword(BaseModel):
    new_password: str


@router.post("/login")
def login(body: Login, conn=Depends(get_conn)):
    token = auth.login(conn, body.username, body.password)
    return {"token": token, "me": me_json(auth.current_member(conn, token))}


@router.post("/logout", status_code=204)
def logout(token: str = Depends(get_token), member=Depends(get_member), conn=Depends(get_conn)):
    auth.logout(conn, token)


@router.get("/me")
def me(member=Depends(get_member)):
    return me_json(member)


@router.put("/me/password", status_code=204)
def change_password(
    body: OwnPassword, token: str = Depends(get_token), member=Depends(get_member), conn=Depends(get_conn)
):
    auth.change_own_password(conn, member["id"], body.current_password, body.new_password, token)


@router.post("/accounts", status_code=201)
def create_account(body: NewAccount, member=Depends(get_member), conn=Depends(get_conn)):
    auth.require(member, "manage_accounts")
    auth.create_account(conn, body.crew_member_id, body.username, body.password)
    return {"crew_member_id": body.crew_member_id, "username": body.username}


@router.put("/accounts/{member_id}/password", status_code=204)
def reset_password(member_id: int, body: ResetPassword, member=Depends(get_member), conn=Depends(get_conn)):
    auth.require(member, "manage_accounts")
    auth.reset_password(conn, member_id, body.new_password)


# --- crew ---


class NewMember(BaseModel):
    name: str
    rank: str


class MemberChange(BaseModel):
    rank: str | None = None
    status: str | None = None


@router.get("/crew")
def list_crew(member=Depends(get_member), conn=Depends(get_conn)):
    return [member_json(row) for row in crew.everyone(conn)]


@router.post("/crew", status_code=201)
def add_member(body: NewMember, member=Depends(get_member), conn=Depends(get_conn)):
    auth.require(member, "manage_crew")
    return member_json(crew.get(conn, crew.add_member(conn, body.name, body.rank)))


@router.patch("/crew/{member_id}")
def change_member(member_id: int, body: MemberChange, member=Depends(get_member), conn=Depends(get_conn)):
    auth.require(member, "manage_crew")
    with db.transaction(conn):
        if body.rank is not None:
            crew.change_rank(conn, member_id, body.rank, member["id"])
        if body.status is not None:
            auth.change_status(conn, member_id, body.status, member["id"])
    return member_json(crew.get(conn, member_id))


# --- voyage ---


class NewVoyage(BaseModel):
    name: str
    destination: str


class NewState(BaseModel):
    state: str


def voyage_json(conn, row) -> dict:
    return {
        "id": row["id"],
        "name": row["name"],
        "destination": row["destination"],
        "state": row["state"],
        "next_states": sorted(voyage.COURSES[row["state"]]),
        "events": [dict(e) for e in voyage.events(conn, row["id"])],
        "roster": [
            {
                "crew_member_id": r["crew_member_id"],
                "name": r["name"],
                "status": r["status"],
                "shares_display": crew.shares_display(r["quarter_shares"]),
            }
            for r in voyage.roster(conn, row["id"])
        ],
    }


@router.get("/voyages")
def list_voyages(member=Depends(get_member), conn=Depends(get_conn)):
    return [voyage_json(conn, row) for row in voyage.everything(conn)]


@router.post("/voyages", status_code=201)
def create_voyage(body: NewVoyage, member=Depends(get_member), conn=Depends(get_conn)):
    auth.require(member, "move_ship")
    voyage_id = voyage.create(conn, body.name, body.destination, member["id"])
    return voyage_json(conn, voyage.get(conn, voyage_id))


@router.get("/voyages/{voyage_id}")
def get_voyage(voyage_id: int, member=Depends(get_member), conn=Depends(get_conn)):
    return voyage_json(conn, voyage.get(conn, voyage_id))


@router.post("/voyages/{voyage_id}/state")
def change_voyage_state(voyage_id: int, body: NewState, member=Depends(get_member), conn=Depends(get_conn)):
    auth.require(member, "move_ship")
    voyage.change_state(conn, voyage_id, body.state, member["id"])
    return voyage_json(conn, voyage.get(conn, voyage_id))


# --- hold ---


class HoldChange(BaseModel):
    item: str
    amount: int
    reason: str


def hold_json(conn) -> dict:
    return {"counts": hold.counts(conn), "weight_stone": hold.weight(conn), "capacity_stone": hold.CAPACITY_STONE}


@router.get("/hold")
def get_hold(member=Depends(get_member), conn=Depends(get_conn)):
    return hold_json(conn)


@router.post("/hold/changes", status_code=201)
def change_hold(body: HoldChange, member=Depends(get_member), conn=Depends(get_conn)):
    auth.require(member, "change_hold")
    hold.change(conn, body.item, body.amount, body.reason, member["id"])
    return hold_json(conn)


# --- loot ---


class NewPlunder(BaseModel):
    amount_reales: int
    description: str


class NewCompensation(BaseModel):
    crew_member_id: int
    injury: str


def plunder_json(conn, voyage_id: int) -> dict:
    voyage.get(conn, voyage_id)
    return {
        "plunder": [
            {"description": p["description"], "at": p["at"], **money("amount", p["amount_reales"])}
            for p in loot.plunder_entries(conn, voyage_id)
        ],
        "compensations": [
            {"name": c["name"], "injury": c["injury"], "at": c["at"], **money("amount", c["amount_reales"])}
            for c in loot.compensation_entries(conn, voyage_id)
        ],
    }


def split_json(conn, row) -> dict:
    return {
        "voyage_id": row["voyage_id"],
        "at": row["at"],
        **money("plunder", row["plunder_reales"]),
        **money("compensation", row["compensation_reales"]),
        **money("leftover", row["leftover_reales"]),
        "lines": [
            {
                "crew_member_id": line["crew_member_id"],
                "name": line["name"],
                "shares_display": crew.shares_display(line["quarter_shares"]),
                "paid_to": line["paid_to"],
                **money("compensation", line["compensation_reales"]),
                **money("share", line["share_reales"]),
                **money("total", line["total_reales"]),
            }
            for line in loot.split_lines(conn, row["id"])
        ],
    }


@router.get("/voyages/{voyage_id}/plunder")
def get_plunder(voyage_id: int, member=Depends(get_member), conn=Depends(get_conn)):
    return plunder_json(conn, voyage_id)


@router.post("/voyages/{voyage_id}/plunder", status_code=201)
def add_plunder(voyage_id: int, body: NewPlunder, member=Depends(get_member), conn=Depends(get_conn)):
    auth.require(member, "manage_loot")
    loot.record_plunder(conn, voyage_id, body.amount_reales, body.description, member["id"])
    return plunder_json(conn, voyage_id)


@router.post("/voyages/{voyage_id}/compensations", status_code=201)
def add_compensation(voyage_id: int, body: NewCompensation, member=Depends(get_member), conn=Depends(get_conn)):
    auth.require(member, "manage_loot")
    loot.record_compensation(conn, voyage_id, body.crew_member_id, body.injury, member["id"])
    return plunder_json(conn, voyage_id)


@router.post("/voyages/{voyage_id}/split", status_code=201)
def run_split(voyage_id: int, member=Depends(get_member), conn=Depends(get_conn)):
    auth.require(member, "manage_loot")
    loot.split(conn, voyage_id, member["id"])
    return split_json(conn, loot.get_split(conn, voyage_id))


@router.get("/voyages/{voyage_id}/split")
def get_split(voyage_id: int, member=Depends(get_member), conn=Depends(get_conn)):
    voyage.get(conn, voyage_id)
    row = loot.get_split(conn, voyage_id)
    if row is None:
        raise ShipError("not_found")
    return split_json(conn, row)


@router.get("/chest")
def get_chest(member=Depends(get_member), conn=Depends(get_conn)):
    return money("balance", loot.chest_balance(conn))

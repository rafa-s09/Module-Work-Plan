"""Every refusal the ship can give. The code is the contract; the message is for humans."""

MESSAGES = {
    "invalid_input": "That order makes no sense, ye bilge rat.",
    "not_found": "No such thing on this ship.",
    "bad_credentials": "Wrong name or password. The gangplank stays up.",
    "locked": "Too many bad tries. The gangplank is up for a quarter of an hour.",
    "not_logged_in": "Board the ship first, stranger.",
    "not_allowed": "Yer rank can't give that order.",
    "weak_password": "A password needs at least 10 characters, matey.",
    "wrong_password": "That's not yer current password.",
    "username_taken": "That name is already on an account.",
    "already_has_account": "That sailor already has an account.",
    "accounts_exist": "This ship already has accounts. Ask the captain.",
    "name_taken": "Someone aboard already goes by that name.",
    "rank_taken": "The ship can't sail with that many officers, or without its captain.",
    "not_aboard": "That one isn't aboard.",
    "bad_status_change": "The articles don't allow that. The dead stay dead.",
    "captain_required": "Make another captain first. A ship needs one.",
    "ship_at_sea": "The ship is at sea. We have but one.",
    "empty_hold": "No water or no rations in the hold. We'd not last a day.",
    "bad_course": "The ship can't sail that course.",
    "overloaded": "Any more and she sinks at the dock.",
    "not_enough": "There's not that much in the hold.",
    "not_plundering": "Plunder is only taken while plundering, ye fool.",
    "not_on_roster": "That one didn't sail on this voyage.",
    "bad_split_time": "The plunder is divided only on the way home or in port.",
    "already_split": "This plunder was divided already, and that's final.",
    "no_plunder": "Nothing to divide. Not a single real.",
    "plunder_too_small": "The compensation is more than the plunder. The quartermaster must decide.",
    "split_broken": "The sums don't match, so nothing was divided.",
}


class ShipError(Exception):
    def __init__(self, code: str):
        super().__init__(code)
        self.code = code
        self.message = MESSAGES[code]

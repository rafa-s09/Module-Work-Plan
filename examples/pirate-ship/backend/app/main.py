"""Hoists the API: the routes, and one shape for every error (api#FR-03 to FR-05)."""

import logging

from fastapi import FastAPI, Request
from fastapi.exceptions import RequestValidationError
from fastapi.responses import JSONResponse
from starlette.exceptions import HTTPException

from app.api import router
from app.errors import MESSAGES, ShipError

log = logging.getLogger("ledger")

STATUS = {
    "invalid_input": 400,
    "weak_password": 400,
    "wrong_password": 400,
    "bad_credentials": 401,
    "not_logged_in": 401,
    "not_allowed": 403,
    "not_found": 404,
    "locked": 423,
}  # any other refusal is 409: the rules said no


def refusal(status: int, code: str) -> JSONResponse:
    return JSONResponse({"error": code, "message": MESSAGES[code]}, status_code=status)


def create_app() -> FastAPI:
    app = FastAPI(title="The Black Ledger")
    app.include_router(router)

    @app.exception_handler(ShipError)
    def ship_error(_: Request, error: ShipError):
        return refusal(STATUS.get(error.code, 409), error.code)

    @app.exception_handler(RequestValidationError)
    def bad_input(_: Request, __: RequestValidationError):
        return refusal(400, "invalid_input")

    @app.exception_handler(HTTPException)
    def http_error(_: Request, error: HTTPException):
        return refusal(404, "not_found") if error.status_code == 404 else refusal(400, "invalid_input")

    @app.exception_handler(Exception)
    def broke(_: Request, error: Exception):
        log.exception("Something broke below deck", exc_info=error)
        return JSONResponse(
            {"error": "server_error", "message": "Blimey! Something broke below deck."}, status_code=500
        )

    return app


app = create_app()

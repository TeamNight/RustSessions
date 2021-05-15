use crate::{SessionData, SessionInner};
use rocket::request::{FromRequest, Outcome};
use rocket::Request;

pub struct CookieSession<'a> {
    data: SessionData<'a>
}

impl FromRequest for CookieSession {
    type Error = ();

    fn from_request(request: &Request) -> Outcome<Self, Self::Error> {
        todo!()
    }
}

impl From<SessionData> for CookieSession {
    fn from<'a>(data: SessionData<'a>) -> Self {
        todo!()
    }
}

impl From<CookieSession> for SessionData {
    fn from<'a>(session: CookieSession<'a>) -> Self {
        todo!()
    }
}

impl Clone for CookieSession {
    fn clone(&self) -> Self {
        CookieSession {
            data: SessionData::clone()
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.data = SessionData::clone(&source.data)
    }
}
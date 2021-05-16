use crate::{SessionData, SessionInner};
use rocket::request::{FromRequest, Outcome};
use rocket::Request;

pub struct CookieSession<'a> {
    data: SessionData<'a>
}

impl FromRequest<'_, '_> for CookieSession<'_> {
    type Error = ();

    fn from_request(request: &Request) -> Outcome<Self, Self::Error> {
        todo!()
    }
}

impl From<&SessionData<'_>> for CookieSession<'_> {
    fn from(data: &SessionData) -> Self {
        todo!()
    }
}

impl From<CookieSession<'_>> for &SessionData<'_> {
    fn from(session: CookieSession) -> Self {
        todo!()
    }
}

impl Clone for CookieSession<'_> {
    fn clone(&self) -> Self {
        CookieSession {
            data: SessionData::clone()
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.data = SessionData::clone(&source.data)
    }
}
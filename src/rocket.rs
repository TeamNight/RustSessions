use crate::{SessionStore, SessionConfig, LocalSessionMap, SessionMap, SessionData, SessionInner, Session};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Rocket, Request, State};
use std::{mem, fmt};
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::sync::{Mutex, PoisonError, MutexGuard, Arc, Weak};
use std::ops::{DerefMut, Deref};
use rocket::request::{FromRequest, Outcome};
use rocket::http::{Status, Cookie};
use chrono::Duration;
use rand::{thread_rng, Rng};
use rand::distributions::Alphanumeric;

fn upcast_arc<T: SessionConfig>(config: Arc<T>) -> Arc<dyn SessionConfig> {
    config
}

pub struct CookieSessionConfig {
    cookie_name: String,
    expiration: Duration,
    _id_gen: Box<dyn Fn() -> String + Send + Sync>,
}

impl CookieSessionConfig {
    pub fn new<F>(cookie_name: &str, expiration_duration: Duration, id_gen: F) -> CookieSessionConfig
    where F: Fn() -> String + Send + Sync + 'static {
        CookieSessionConfig {
            cookie_name: cookie_name.to_string(),
            expiration: expiration_duration,
            _id_gen: Box::new(id_gen)
        }
    }

    pub fn cookie_name(&self) -> &str {
         self.cookie_name.as_str()
    }
}

impl SessionConfig for CookieSessionConfig {
    fn expiration_duration(&self) -> Option<Duration> {
        if self.expiration.is_zero() {
            None
        } else {
            Some(self.expiration.clone())
        }
    }

    fn id_gen(&self) -> &dyn Fn() -> String {
        return self._id_gen.as_ref()
    }
}

impl Default for CookieSessionConfig {
    fn default() -> Self {
        CookieSessionConfig {
            cookie_name: "session".to_string(),
            expiration: Duration::hours(1),
            _id_gen: Box::new(|| -> String {
                thread_rng().sample_iter(&Alphanumeric).take(24).map(char::from).collect::<String>()
            })
        }
    }
}

///
///
/// CookieSession
///
///
pub struct CookieSession {
    data: SessionData
}

impl Session for CookieSession {
    fn access<F: FnMut(&mut SessionInner)>(&self, mut closure: F) -> Result<(), PoisonError<MutexGuard<SessionInner>>> {
        match self.data.lock() {
            Ok(mut inner) => {
                closure(&mut *inner);
                Ok(())
            },
            Err(err) => Err(err)
        }
    }

    fn data_ref(&self) -> &SessionData {
        &self.data
    }
}

impl FromRequest<'_, '_> for CookieSession {
    type Error = ();

    fn from_request(request: &Request) -> Outcome<Self, Self::Error> {
        let cookie_config = request.guard::<State<Arc<CookieSessionConfig>>>()?;
        let session_store = request.guard::<State<SessionStore>>()?;

        //What needs to be implemented is:
        //1. CookieSessionConfig with cookie name </
        //2. Retrieve CookieSessionConfig as Request Guard </
        //3. Check for cookie with cookie name and retrieve it </
        //4a. If Cookie is None => create new session, save it and send cookie </
        //4b. If Cookie is Some => retrieve it, get session </
        //4ba. If Session is Some => return it </
        //4ba. If Session is None => delete cookie and get back to from_request </

        match request.cookies().get(cookie_config.cookie_name()) {
            None => {
                let generate = cookie_config.id_gen();
                let id: String = generate();

                let session = CookieSession {
                    data: SessionData::new(Mutex::new(SessionInner::new(id.as_str(), cookie_config.clone()))),
                };

                session_store.insert(id.as_str(), &session);

                Outcome::Success(session)
            },
            Some(cookie) => {
                let session = session_store.get::<CookieSession>(cookie.value());

                match session {
                    Some(cookie_session) => {
                        Outcome::Success(cookie_session)
                    },
                    None => {
                        request.cookies().remove(Cookie::named(cookie.name().to_owned()));

                        Self::from_request(request)
                    }
                }
            }
        }
    }
}

impl From<SessionData> for CookieSession {
    fn from(data: SessionData) -> Self {
        CookieSession {
            data
        }
    }
}

impl Clone for CookieSession {
    fn clone(&self) -> Self {
        CookieSession {
            data: SessionData::clone(&self.data)
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.data = SessionData::clone(&source.data)
    }
}

///
///
/// Configuration for fairing
///
///


pub struct Sessions {
    store: SessionStore,
    configs: Vec<Box<dyn SessionConfig>>
}

impl Sessions {
    fn new() -> Sessions {
        Sessions {
            store: SessionStore::new(LocalSessionMap::new()),
            configs: vec![]
        }
    }

    fn config<T: SessionConfig>(mut self, config: T) -> Sessions {
        self.configs.push(Box::new(config));

        self
    }

    fn session_map<T: SessionMap>(mut self, map: T) -> Sessions {
        self.store.inner = Box::new(map);

        self
    }

    fn fairing(self) -> SessionFairing {
        SessionFairing { inner: Mutex::new(self) }
    }
}

///
///
/// Fairing
///
///

pub struct SessionFairing {
    inner: Mutex<Sessions>
}

impl Fairing for SessionFairing {
    fn info(&self) -> Info {
        Info {
            name: "Sessions",
            kind: Kind::Attach
        }
    }

    fn on_attach(&self, rocket: Rocket) -> Result<Rocket, Rocket> {
        //add session store to managed state as well as all session configs
        let mut sessions = self.inner.lock().unwrap();
        let store = mem::take(&mut sessions.store);

        let mut rocket = rocket.manage(store);

        for i in 0..sessions.configs.len() {
            rocket = rocket.manage(sessions.configs.remove(i));
        }

        Ok(rocket)
    }
}
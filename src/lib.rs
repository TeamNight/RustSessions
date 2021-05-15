use chrono::{DateTime, Utc, MIN_DATETIME};
use std::any::Any;
use std::collections::HashMap;
use rocket::request::FromRequest;
use std::sync::{Arc, Mutex, RwLock};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::Rocket;

pub mod cookie;

pub trait Session {
    /// Returns the id of the session
    fn id(&self) -> &str;

    /// Sets the id of the session
    ///
    /// # Arguments
    ///
    /// * `new_id` - String slice for new id
    ///
    fn set_id(&mut self, new_id: &str);

    /// Returns the creation time of the session
    fn creation_time(&self) -> &DateTime<Utc>;

    /// Returns the time the user last accessed using the session
    fn last_accessed(&self) -> &DateTime<Utc>;

    /// Returns the time the session will expire or
    /// [`chrono::MIN_DATETIME`] if the session never expires
    fn expires(&self) -> &DateTime<Utc>;

    /// Updates the last accessed and expiration time
    fn update_time(&mut self);

    /// Inserts a new attribute to the session
    ///
    /// # Arguments
    ///
    /// * `key` - Unique string key for the attribute
    /// * `attribute` - the attribute
    ///
    fn insert<T>(&mut self, key: &str, attribute: T) where T: Serialize + Any;

    /// Removes an attribute by its key and returns
    /// the removed attribute if there was one
    ///
    /// # Arguments
    ///
    /// * `key` - The key of the attribute to remove
    ///
    fn remove(&mut self, key: &str) -> Option<Box<dyn Any>>;

    /// Returns the attribute behind the given key
    ///
    /// # Arguments
    ///
    /// * `key` - The key of the attribute to return
    ///
    fn get_attribute<T>(&self, key: &str) -> Option<&T> where T: Serialize + Any;
}

pub trait SessionConfig {
    /// Returns the duration until a session expires
    fn expiration_duration(&self) -> Option<chrono::Duration>;
}

pub struct SessionInner<'a> {
    id: String,
    created: DateTime<Utc>,
    last_accessed: DateTime<Utc>,
    expires: DateTime<Utc>,
    attributes: HashMap<String, Box<dyn Any>>,
    _config: &'a dyn SessionConfig
}

impl<'a> SessionInner<'a> {
    pub fn new(id: &str, config: &'a dyn SessionConfig) -> SessionInner {
        let creation_time = Utc::now();

        SessionInner {
            id: id.to_string(),
            created: creation_time,
            last_accessed: creation_time,
            expires: match config.expiration_duration() {
                Some(duration) => {
                    creation_time + duration
                },
                None => chrono::MIN_DATETIME
            },
            attributes: Default::default(),
            _config: config
        }
    }
}

impl Session for SessionInner {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    fn set_id(&mut self, new_id: &str) {
        self.id = new_id.to_string();
    }

    fn creation_time(&self) -> &DateTime<Utc> {
        &self.created
    }

    fn last_accessed(&self) -> &DateTime<Utc> {
        &self.last_accessed
    }

    fn expires(&self) -> &DateTime<Utc> {
        &self.expires
    }

    fn update_time(&mut self) {
        self.last_accessed = Utc::now();

        if let Some(duration) = self._config.expiration_duration() {
            self.expires = self.last_accessed + duration
        }
    }

    fn insert<T>(&mut self, key: &str, attribute: T)
        where T: Serialize + Any {
        self.attributes.insert(key.to_string(), Box::new(attribute));
    }

    fn remove(&mut self, key: &str) -> Option<Box<dyn Any>> {
        self.attributes.remove(key)
    }

    fn get_attribute<T>(&self, key: &str) -> Option<&T>
        where T: Serialize + Any {
        let result = self.attributes.get(key);

        match result {
            Some(data) => {
                return data.downcast_ref()
            },
            None => None
        }
    }
}

/// A map containing all sessions as [`SessionData`]
pub trait SessionMap {
    /// Lists all sessions in this SessionMap
    fn list(&self) -> Vec<&dyn Session>;

    /// Gets a session by its id
    ///
    /// # Arguments
    ///
    /// * `id` - A string slice containing the id
    ///
    fn get(id: &str) -> &SessionData;

    /// Inserts a session into the map
    ///
    /// # Arguments
    ///
    /// * `id` - The unique id of the session
    /// * `session` - A session data object to be moved into the map
    ///
    fn insert(id: &str, session: SessionData);

    /// Removes a session by its id and returns true if there was a session removed
    ///
    /// # Arguments
    ///
    /// * `id` - The unique id of the session
    ///
    fn remove(id: &str) -> bool;

    /// Removes expired sessions and returns the count of expired sessions
    fn remove_expired() -> usize;

    /// Clears the session map, invalidating all sessions.
    ///
    /// The map needs to call invalidate on all sessions before
    /// removing them from the underlying map.
    fn clear();
}

type SessionData<'a> = Arc<Mutex<SessionInner<'a>>>;

pub struct LocalSessionMap<'a> {
    sessions: RwLock<HashMap<String, SessionData<'a>>>
}

impl<'a> LocalSessionMap<'a> {
    pub fn new() -> LocalSessionMap {
        LocalSessionMap {
            sessions: RwLock::new(HashMap::new())
        }
    }
}

impl<'a> SessionMap for LocalSessionMap<'a> {
    fn list(&self) -> Vec<&dyn Session> {
        todo!()
    }

    fn get(id: &str) -> &SessionData<'a> {
        todo!()
    }

    fn insert(id: &str, session: SessionData<'a>) {
        todo!()
    }

    fn remove(id: &str) -> bool {
        todo!()
    }

    fn remove_expired() -> usize {
        todo!()
    }

    fn clear() {
        todo!()
    }
}

struct SessionStore {
    inner: Box<dyn SessionMap>
}

impl SessionStore {
    /// Lists all sessions in this SessionStore
    fn list(&self) -> Vec<&dyn Session> {
        vec![]
    }

    /// Gets a session by its id
    ///
    /// # Arguments
    ///
    /// * `id` - A string slice containing the id
    ///
    fn get<T>(id: &str) -> Option<T> where T: Session + Clone + From<SessionData> {
        None
    }

    /// Inserts a session into the map
    ///
    /// # Arguments
    ///
    /// * `id` - The unique id of the session
    /// * `session` - A struct implementing the [`Session`], [`Clone`] and [`Into<SessionData>`]
    ///               traits
    ///
    fn insert<T>(id: &str, session: T) where T: Session + Clone + Into<SessionData> {

    }

    /// Removes a session by its id and returns true if there was a session removed
    ///
    /// # Arguments
    ///
    /// * `id` - The unique id of the session
    ///
    fn remove(id: &str) -> bool {
        false
    }

    /// Removes expired sessions and returns the count of expired sessions
    fn remove_expired() -> usize {
        0
    }

    /// Clears the session map, invalidating all sessions.
    ///
    /// The map needs to call invalidate on all sessions before
    /// removing them from the underlying map.
    fn clear() {

    }
}

///
///
/// Configuration for fairing
///
///


pub struct Sessions {
    store: SessionStore,
    configs: Vec<dyn SessionConfig>
}

impl Sessions {
    fn new() -> Sessions {
        Sessions {
            store: SessionStore::new(LocalSessionMap::new()),
            configs: vec![]
        }
    }

    fn config<T: SessionConfig>(mut self, config: T) -> Sessions {
        self.configs.push(config);

        self
    }

    fn session_map<T: SessionMap>(mut self, map: T) -> Sessions {
        self
    }

    fn fairing(self) -> SessionFairing {
        SessionFairing { inner: self }
    }
}

///
///
/// Fairing
///
///

pub struct SessionFairing {
    inner: Sessions
}

#[rocket::async_trait]
impl Fairing for SessionFairing {
    fn info(&self) -> Info {
        Info {
            name: "Sessions",
            kind: Kind::Attach
        }
    }

    fn on_attach(&self, rocket: Rocket) -> Result<Rocket, Rocket> {
        //add session store to managed state as well as all session configs
        todo!()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
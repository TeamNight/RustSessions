use chrono::{DateTime, Utc};
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use rocket::fairing::{Fairing, Info, Kind};
use rocket::Rocket;
use std::borrow::Borrow;
use erased_serde::Serialize;

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

    /// Returns true if the session is expired
    fn is_expired(&self) -> bool;

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
    fn remove(&mut self, key: &str) -> Option<Box<dyn Serialize>>;

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
    pub fn new(id: &str, config: &'a dyn SessionConfig) -> SessionInner<'a> {
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

impl<'a> Session for SessionInner<'a> {
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

    fn is_expired(&self) -> bool {
        let current_time = Utc::now();

        if self.expires() <= &current_time {
            true
        } else {
            false
        }
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
    fn list(&self) -> Vec<&SessionData>;

    /// Gets a session by its id
    ///
    /// # Arguments
    ///
    /// * `id` - A string slice containing the id
    ///
    fn get(&self, id: &str) -> Option<&SessionData>;

    /// Inserts a session into the map
    ///
    /// # Arguments
    ///
    /// * `id` - The unique id of the session
    /// * `session` - A session data object to be moved into the map
    ///
    fn insert(&self, id: &str, session: SessionData);

    /// Removes a session by its id and returns true if there was a session removed
    ///
    /// # Arguments
    ///
    /// * `id` - The unique id of the session
    ///
    fn remove(&self, id: &str) -> bool;

    /// Removes expired sessions and returns the count of expired sessions
    fn remove_expired(&self) -> usize;

    /// Clears the session map, invalidating all sessions.
    ///
    /// The map needs to call invalidate on all sessions before
    /// removing them from the underlying map.
    fn clear(&self);
}

type SessionData<'a> = Arc<Mutex<SessionInner<'a>>>;

pub struct LocalSessionMap<'a> {
    sessions: RwLock<HashMap<String, SessionData<'a>>>
}

impl<'a> LocalSessionMap<'a> {
    pub fn new() -> LocalSessionMap<'a> {
        LocalSessionMap {
            sessions: RwLock::new(HashMap::new())
        }
    }
}

impl<'a> SessionMap for LocalSessionMap<'a> {
    fn list(&self) -> Vec<&SessionData<'a>> {
        match self.sessions.read() {
            Ok(map) => map.values().collect(),
            Err(e) => panic!("Unable to acquire session map lock, {}", e)
        }
    }

    fn get(&self, id: &str) -> Option<&SessionData<'a>> {
        match self.sessions.read() {
            Ok(map) => map.get(id),
            Err(e) => panic!("Unable to acquire session map lock: {}", e)
        }
    }

    fn insert(&self, id: &str, session: SessionData<'a>) {
        match self.sessions.write() {
            Ok(mut map) => map.insert(id.to_string(), session),
            Err(e) => panic!("Unable to acqurie session map write lock: {}", e)
        };
    }

    fn remove(&self, id: &str) -> bool {
        match self.sessions.write() {
            Ok(mut map) => {
                match map.remove(id) {
                    None => false,
                    Some(_) => true
                }
            },
            Err(e) => panic!("Unable to acqurie session map write lock: {}", e)
        }
    }

    fn remove_expired(&self) -> usize {
        let mut removed: usize = 0;

        match self.sessions.write() {
            Ok(mut map) => {
                for val in map.values() {
                    match val.lock() {
                        Ok(data) => {
                            if data.is_expired() {
                                map.remove(data.id());
                                removed += 1;
                            }
                        }
                        Err(err) => todo!()
                    }
                }
            },
            Err(e) => panic!("Unable to acqurie session map write lock: {}", e)
        };

        removed
    }

    fn clear(&self) {
        match self.sessions.write() {
            Ok(mut map) => {
                for val in map.values() {
                    match val.lock() {
                        Ok(data) => map.remove(data.id()),
                        Err(err) => todo!()
                    }
                }
            },
            Err(e) => panic!("Unable to acquire session map write lock: {}", e)
        };
    }
}

struct SessionStore {
    inner: Box<dyn SessionMap>
}

impl SessionStore {
    /// Lists all sessions in this SessionStore
    fn list(&self) -> Vec<&SessionData> {
        self.inner.borrow().list()
    }

    /// Gets a session by its id
    ///
    /// # Arguments
    ///
    /// * `id` - A string slice containing the id
    ///
    fn get<'a, T>(&self, id: &str) -> Option<T> where T: Session + Clone + From<&'a SessionData<'a>> {
        let session_data = self.inner.borrow().get(id);

        if let Some(data) = session_data {
            Some(T::from(data))
        } else {
            None
        }
    }

    /// Saves a session to the store
    ///
    /// # Arguments
    ///
    /// * `id` - The unique id of the session
    /// * `session` - A struct implementing the [`Session`], [`Clone`] and [`Into<SessionData>`]
    ///               traits
    ///
    fn insert<'a, T>(&self, id: &str, session: T) where T: Session + Clone + Into<&'a SessionData<'a>> {
        let session_data: &SessionData = session.into();

        self.inner.borrow().insert(id, session_data.clone())
    }

    /// Removes a session by its id and returns true if there was a session removed
    ///
    /// # Arguments
    ///
    /// * `id` - The unique id of the session
    ///
    fn remove(&self, id: &str) -> bool {
        self.inner.borrow().remove(id)
    }

    /// Removes expired sessions and returns the count of expired sessions
    fn remove_expired(&self) -> usize {
        self.inner.borrow().remove_expired()
    }

    /// Clears the session store, invalidating all sessions.
    ///
    /// The underlying map needs to call invalidate on all sessions before
    /// removing them.
    fn clear(&self) {
        self.inner.borrow().clear();
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
        self.store.inner = map;

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
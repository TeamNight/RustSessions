use chrono::{DateTime, Utc};
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::borrow::Borrow;
use erased_serde::Serialize;

/// Trait to upcast Attribute to Any and use [`std::any::Any.downcast_ref(&self)`]
pub trait AsAny {
    fn as_any_ref(&self) -> &dyn Any;
}

/// AsAny is implemented for all structs implementing Any
impl<T: Any> AsAny for T {
    fn as_any_ref(&self) -> &dyn Any {
        self
    }
}

/// Attribute is the type representing data in a session. It requires the struct to implement Any,
/// Serialize, Send and Sync.
trait Attribute: AsAny + Serialize + Send + Sync + 'static {}

/// Attribute is automatically implemented for the named traits
impl<T> Attribute for T where T: AsAny + Serialize + Send + Sync + 'static {}

///A session that can be created by the framework in case it needs session data
pub trait Session {
    /// Accesses the session data behind this session and acquires the lock protecting the data
    /// of the session.
    ///
    /// All access, mutable and immutable shall be done in this method.
    ///
    /// # Arguments
    ///
    /// * `closure` - The method to access the session data
    ///
    fn access<F: FnMut(&SessionInner)>(closure: F);

    /// Invalidates the session by setting expiration date to [`Session.creation_time()`]
    fn invalidate(&mut self);
}

/// Configuration for sessions, such as duration until the session expires
pub trait SessionConfig: Send + Sync + 'static {
    /// Returns the duration until a session expires
    fn expiration_duration(&self) -> Option<chrono::Duration>;

    /// Returns an id generator for ids of the session
    fn id_gen(&self) -> &dyn Fn() -> String;
}

/// Struct holding the real session data
pub struct SessionInner {
    id: String,
    created: DateTime<Utc>,
    last_accessed: DateTime<Utc>,
    expires: DateTime<Utc>,
    attributes: HashMap<String, Box<dyn Attribute>>,
    _config: &'static dyn SessionConfig
}

impl SessionInner {
    /// Creates a new [`SessionInner`] with the given id and session config
    ///
    /// # Arguments
    ///
    /// * `id` - The id of the session
    /// * `config` - The session config for this session type
    pub fn new(id: &str, config: &'static dyn SessionConfig) -> SessionInner {
        let creation_time = Utc::now();

        SessionInner {
            id: id.to_string(),
            created: creation_time,
            last_accessed: creation_time.clone(),
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

    /// Returns the id of the session
    fn id(&self) -> &str {
        self.id.as_str()
    }

    /// Sets the id of the session
    ///
    /// # Arguments
    ///
    /// * `new_id` - String slice for new id
    ///
    fn set_id(&mut self, new_id: &str) {
        self.id = new_id.to_string();
    }

    /// Returns the creation time of the session
    fn creation_time(&self) -> &DateTime<Utc> {
        &self.created
    }

    /// Returns the time the user last accessed using the session
    fn last_accessed(&self) -> &DateTime<Utc> {
        &self.last_accessed
    }

    /// Returns the time the session will expire or
    /// [`chrono::MIN_DATETIME`] if the session never expires
    fn expires(&self) -> &DateTime<Utc> {
        &self.expires
    }

    /// Returns true if the session is expired
    fn is_expired(&self) -> bool {
        let current_time = Utc::now();

        if self.expires() <= &current_time {
            true
        } else {
            false
        }
    }

    /// Updates the last accessed and expiration time
    fn update_time(&mut self) {
        self.last_accessed = Utc::now();

        if let Some(duration) = self._config.expiration_duration() {
            self.expires = self.last_accessed + duration
        }
    }

    /// Invalidates the session by setting expiration date to [`Session.creation_time()`]
    fn invalidate(&mut self) {
        self.expires = self.created.clone();
    }

    /// Inserts a new attribute to the session
    ///
    /// # Arguments
    ///
    /// * `key` - Unique string key for the attribute
    /// * `attribute` - the attribute
    ///
    fn insert<T:>(&mut self, key: &str, attribute: T)
        where T: Attribute {
        self.attributes.insert(key.to_string(), Box::new(attribute));
    }

    /// Removes an attribute by its key and returns
    /// the removed attribute if there was one
    ///
    /// # Arguments
    ///
    /// * `key` - The key of the attribute to remove
    ///
    fn remove(&mut self, key: &str) -> Option<Box<dyn Attribute>> {
        self.attributes.remove(key)
    }

    /// Returns the attribute behind the given key
    ///
    /// # Arguments
    ///
    /// * `key` - The key of the attribute to return
    ///
    fn get_attribute<T>(&self, key: &str) -> Option<&T>
        where T: Attribute {
        let result = self.attributes.get(key);

        match result {
            Some(data) => {
                let attr = data.as_any_ref();
                if attr.is::<T>() {
                    return attr.downcast_ref::<T>();
                } else {
                    None
                }
            },
            None => None
        }
    }
}

/// A map containing all sessions as [`SessionData`]
///
/// The map chooses where to save session data and retrieve it, e.g. in RAM or Redis
pub trait SessionMap: Send + Sync + 'static {
    /// Lists all sessions in this SessionMap
    fn list(&self, closure: &dyn Fn(Vec<&SessionData>));

    /// Gets a session by its id
    ///
    /// # Arguments
    ///
    /// * `id` - A string slice containing the id
    ///
    fn get(&self, id: &str) -> Option<SessionData>;

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

/// The type to protect session data from data races
type SessionData = Arc<Mutex<SessionInner>>;

/// A session map to save session data in RAM
pub struct LocalSessionMap {
    sessions: RwLock<HashMap<String, SessionData>>
}

impl LocalSessionMap {
    pub fn new() -> LocalSessionMap {
        LocalSessionMap {
            sessions: RwLock::new(HashMap::new())
        }
    }
}

impl SessionMap for LocalSessionMap {
    fn list(&self, closure: &dyn Fn(Vec<&SessionData>)) {
        match self.sessions.read() {
            Ok(map) => closure(map.values().collect()),
            Err(e) => panic!("Unable to acquire session map lock, {}", e)
        }
    }

    fn get(&self, id: &str) -> Option<SessionData> {
        match self.sessions.read() {
            Ok(map) => {
                if map.contains_key(id) {
                    return Some(map.get(id).unwrap().clone());
                }
                None
            },
            Err(e) => panic!("Unable to acquire session map lock: {}", e)
        }
    }

    fn insert(&self, id: &str, session: SessionData) {
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
                map.retain(|key, data| -> bool {
                    match data.lock() {
                        Ok(inner) => {
                            if inner.is_expired() {
                                removed += 1;
                                return true;
                            }
                            return false
                        },
                        Err(err) => false
                    }
                });
            },
            Err(e) => panic!("Unable to acqurie session map write lock: {}", e)
        };

        removed
    }

    fn clear(&self) {
        match self.sessions.write() {
            Ok(mut map) => {
                map.retain(|key, val| {
                    match val.lock() {
                        Ok(mut data) => data.invalidate(),
                        Err(err) => todo!(),
                    };
                    false
                })
            },
            Err(e) => panic!("Unable to acquire session map write lock: {}", e)
        };
    }
}

/// Session Store is the public interface for the framework to access sessions.
struct SessionStore {
    inner: Box<dyn SessionMap>
}

impl SessionStore {
    /// Creates a new session store
    ///
    /// # Arguments
    ///
    /// * `map` - The session map used by the store
    ///
    pub fn new<T: SessionMap>(map: T) -> SessionStore {
        SessionStore {
            inner: Box::new(map),
        }
    }
    /// Lists all sessions in this SessionStore
    fn list<F: Fn(Vec<&SessionData>)>(&self, closure: F)  {
        let map: &dyn SessionMap = self.inner.borrow();
        map.list(&closure);
    }

    /// Gets a session by its id
    ///
    /// The implementation of [`From<SessionData>`] gets passed a clone of [`SessionData`].
    ///
    /// # Arguments
    ///
    /// * `id` - A string slice containing the id
    ///
    fn get<'a, T>(&'a self, id: &str) -> Option<T> where T: Session + Clone + From<SessionData> {
        let map: &'a dyn SessionMap = self.inner.borrow();
        let session_data = map.get(id);

        if let Some(data) = session_data {
            match data.lock() {
                Ok(inner) => {
                    if inner.is_expired() {
                        return None;
                    }

                    Some(T::from(data.clone()))
                }
                Err(err) => {
                    /// TODO: add error message
                    /// error!("Removing session {} from session map due to {}", id, err)
                    None
                }
            }
        } else {
            None
        }
    }

    /// Saves a session to the store.
    ///
    /// Inside of this method a clone of the SessionData occurs.
    ///
    /// # Arguments
    ///
    /// * `id` - The unique id of the session
    /// * `session` - A struct implementing the [`Session`], [`Clone`] and [`Into<SessionData>`]
    ///               traits
    ///
    fn insert<'a, T>(&self, id: &str, session: &'a T) where T: Session + Clone + Into<&'a SessionData> {
        let session_data: &SessionData = session.into();

        let map: &dyn SessionMap = self.inner.borrow();
        map.insert(id, session_data.clone())
    }

    /// Removes a session by its id and returns true if there was a session removed
    ///
    /// # Arguments
    ///
    /// * `id` - The unique id of the session
    ///
    fn remove(&self, id: &str) -> bool {
        let map: &dyn SessionMap = self.inner.borrow();
        map.remove(id)
    }

    /// Removes expired sessions and returns the count of expired sessions
    fn remove_expired(&self) -> usize {
        let map: &dyn SessionMap = self.inner.borrow();
        map.remove_expired()
    }

    /// Clears the session store, invalidating all sessions.
    ///
    /// The underlying map needs to call invalidate on all sessions before
    /// removing them.
    fn clear(&self) {
        let map: &dyn SessionMap = self.inner.borrow();
        map.clear();
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        SessionStore::new(LocalSessionMap::new())
    }
}

#[cfg(feature = "rocket")]
mod rocket;

#[cfg(feature = "rocket")]
pub use crate::rocket::*;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}

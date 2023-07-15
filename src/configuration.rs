use crate::error::Error;
use shuttle_secrets::SecretStore;

#[derive(Debug)]
pub struct Settings {
    pub database: DatabaseSettings,
}

#[derive(Debug)]
pub struct DatabaseSettings {
    pub host: String,
    pub port: String,
    pub secure: bool,
    pub username: String,
    pub password: String,
    pub ns: String,
    pub db: String,
}

impl TryFrom<&SecretStore> for Settings {
    type Error = Error;

    fn try_from(store: &SecretStore) -> Result<Self, Self::Error> {
        let scheme = get_secret(store, "DB_SCHEME")?;
        let secure = if scheme == "https" { true } else { false };

        let settings = Settings {
            database: DatabaseSettings {
                host: get_secret(store, "DB_HOST")?,
                port: get_secret(store, "DB_PORT")?,
                secure,
                username: get_secret(store, "DB_USERNAME")?,
                password: get_secret(store, "DB_PASSWORD")?,
                ns: get_secret(store, "DB_NS")?,
                db: get_secret(store, "DB_DB")?,
            },
        };

        Ok(settings)
    }
}

impl DatabaseSettings {
    pub fn get_connection_string(&self) -> String {
        let mut connection_string = self.host.clone();

        if self.port != "80" {
            connection_string = format!("{}:{}", connection_string, self.port);
        }

        connection_string
    }
}

fn get_secret(store: &SecretStore, env_var: &str) -> Result<String, Error> {
    match store.get(env_var) {
        Some(secret) => Ok(secret),
        None => Err(Error::MissingEnvVar),
    }
}

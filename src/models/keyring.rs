use crate::config;
use secret_service::{Collection, EncryptionType, Error, SecretService};
use sha2::{Digest, Sha512};
use std::collections::HashMap;

pub struct Keyring;

fn token_attributes(token_id: &str) -> HashMap<&str, &str> {
    let mut attributes = HashMap::new();
    attributes.insert("application", config::APP_ID);
    attributes.insert("type", "token");
    attributes.insert("token_id", &token_id);
    attributes
}

fn password_attributes() -> HashMap<&'static str, &'static str> {
    let mut attributes = HashMap::new();
    attributes.insert("application", config::APP_ID);
    attributes.insert("type", "password");
    attributes
}

impl Keyring {
    pub fn get_default_collection<'a>(ss: &'a SecretService<'a>) -> Result<Collection<'a>, Error> {
        let collection = match ss.get_default_collection() {
            Err(Error::NoResult) => ss.create_collection("default", "default"),
            e => e,
        }?;

        Ok(collection)
    }

    pub fn ensure_unlocked() -> Result<(), Error> {
        let ss = secret_service::SecretService::new(secret_service::EncryptionType::Dh)?;
        let collection = Keyring::get_default_collection(&ss)?;
        collection.ensure_unlocked()?;

        Ok(())
    }

    pub fn store(label: &str, token: &str) -> Result<String, Error> {
        let token = token.as_bytes();
        let ss = SecretService::new(EncryptionType::Dh)?;
        let col = Self::get_default_collection(&ss)?;

        let token_id = hex::encode(Sha512::digest(token));
        let attributes = token_attributes(&token_id);
        let base64_token = hex::encode(token);
        col.create_item(label, attributes, base64_token.as_bytes(), true, "plain")?;
        Ok(token_id)
    }

    pub fn token(token_id: &str) -> Result<Option<String>, Error> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let col = Self::get_default_collection(&ss)?;

        let attributes = token_attributes(token_id);
        let items = col.search_items(attributes)?;
        Ok(match items.get(0) {
            Some(e) => Some(String::from_utf8(hex::decode(e.get_secret()?).unwrap()).unwrap()),
            _ => None,
        })
    }

    pub fn remove_token(token_id: &str) -> Result<(), Error> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let col = Self::get_default_collection(&ss)?;

        let attributes = token_attributes(token_id);
        let items = col.search_items(attributes)?;
        match items.get(0) {
            Some(e) => e.delete(),
            _ => Err(Error::NoResult),
        }
    }

    pub fn has_set_password() -> Result<bool, Error> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let col = Self::get_default_collection(&ss)?;

        let attributes = password_attributes();
        match col.search_items(attributes) {
            Ok(items) => Ok(matches!(items.get(0), Some(_))),
            _ => Ok(false),
        }
    }

    pub fn set_password(password: &str) -> Result<(), Error> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let col = Self::get_default_collection(&ss)?;

        let attributes = password_attributes();
        col.create_item(
            "Authenticator password",
            attributes,
            password.as_bytes(),
            true,
            "plain",
        )?;
        Ok(())
    }

    pub fn reset_password() -> Result<(), Error> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let col = Self::get_default_collection(&ss)?;

        let attributes = password_attributes();
        let items = col.search_items(attributes)?;

        match items.get(0) {
            Some(i) => i.delete(),
            None => Err(Error::NoResult),
        }
    }

    pub fn is_current_password(password: &str) -> Result<bool, Error> {
        let ss = SecretService::new(EncryptionType::Dh)?;
        let col = Self::get_default_collection(&ss)?;

        let attributes = password_attributes();
        let items = col.search_items(attributes)?;
        Ok(match items.get(0) {
            Some(i) => i.get_secret()? == password.as_bytes(),
            None => false,
        })
    }
}

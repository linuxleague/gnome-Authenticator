//! Aegis Import/Export Module
//!
//! See <https://github.com/beemdevelopment/Aegis/blob/master/docs/vault.md> for a description of the
//! aegis vault format.
//!
//! This module does not convert all information from aegis (note, icon, group
//! are lost). When exporting to the aegis json format the icon, url, help url,
//! and tags are lost.
//!
//! Exported files by this module cannot be decrypted by the python script
//! provided in the aegis repository (<https://github.com/beemdevelopment/Aegis/blob/master/docs/decrypt.py>). However,
//! aegis android app is able to read the files! See line 173 for a discussion.

use aes_gcm::{aead::Aead, NewAead};
use anyhow::{Context, Result};
use gettextrs::gettext;
use gtk::prelude::*;
use rand::RngCore;
use serde::{Deserialize, Serialize};

use super::{Backupable, Restorable, RestorableItem};
use crate::models::{Account, Algorithm, OTPMethod, Provider, ProvidersModel};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Aegis {
    Encrypted(AegisEncrypted),
    Plaintext(AegisPlainText),
}

/// Plaintext version of the JSON format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AegisPlainText {
    version: u32,
    header: Header,
    db: Database,
}

impl Default for AegisPlainText {
    fn default() -> Self {
        Self {
            version: 1,
            header: Header {
                params: None,
                slots: Default::default(),
            },
            db: Default::default(),
        }
    }
}

/// Encrypted version of the JSON format. `db` is simply a base64 encoded string
/// with an encrypted AegisDatabase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AegisEncrypted {
    version: u32,
    header: Header,
    db: String,
}

impl Default for Aegis {
    fn default() -> Self {
        Self::Plaintext(AegisPlainText::default())
    }
}

impl Aegis {
    pub fn add_item(&mut self, item: Item) {
        if let Self::Plaintext(plain_text) = self {
            plain_text.db.entries.push(item);
        } else {
            // This is an implementation error. Thus, panic is here okay.
            panic!("Trying to add an OTP item to an encrypted aegis database")
        }
    }

    pub fn encrypt(&mut self, password: &str) -> Result<()> {
        // Create a new master key
        let mut rng = rand::thread_rng();
        let mut master_key = [0u8; 32];
        rng.fill_bytes(&mut master_key);

        // Create a new header (including defaults for a password slot)
        let mut header = Header {
            params: Some(HeaderParam::default()),
            slots: Some(vec![HeaderSlot::default()]),
        };

        // We only support password encrypted database so far so we don't have to do any
        // checks for the slot type
        let mut password_slot = &mut header.slots.as_mut().unwrap().get_mut(0).unwrap();
        // Derive key from given password
        let mut derived_key: [u8; 32] = [0u8; 32];
        let params = scrypt::Params::new(
            // TODO log2 for u64 is not stable yet. Change this in the future.
            (password_slot.n() as f64).log2() as u8,
            password_slot.r(),
            password_slot.p(),
        )
        // All parameters are default values. Thus, this should always work and unwrap is okay.
        .expect("Scrypt params creation");
        scrypt::scrypt(
            password.as_bytes(),
            password_slot.salt(),
            &params,
            &mut derived_key,
        )
        .map_err(|_| anyhow::anyhow!("Scrypt key derivation"))?;

        // Encrypt new master key with derived key
        let cipher = aes_gcm::Aes256Gcm::new(aes_gcm::Key::from_slice(&derived_key));
        let mut ciphertext: Vec<u8> = cipher
            .encrypt(
                aes_gcm::Nonce::from_slice(&password_slot.key_params.nonce),
                master_key.as_ref(),
            )
            .map_err(|_| anyhow::anyhow!("Encrypter master key"))?;

        // Add encrypted master key and tag to our password slot. If this assignment
        // fails, we have a mistake in our logic, thus unwrap is okay.
        password_slot.key_params.tag = ciphertext.split_off(32).try_into().unwrap();
        password_slot.key = ciphertext.try_into().unwrap();

        // Finally, we get the JSON string for the database and encrypt it.
        if let Self::Plaintext(plain_text) = self {
            let db_json: Vec<u8> = serde_json::ser::to_string_pretty(&plain_text.db)?
                .as_bytes()
                .to_vec();
            let cipher = aes_gcm::Aes256Gcm::new(aes_gcm::Key::from_slice(&master_key));
            let mut ciphertext: Vec<u8> = cipher
                .encrypt(
                    aes_gcm::Nonce::from_slice(&header.params.as_ref().unwrap().nonce),
                    db_json.as_ref(),
                )
                .map_err(|_| anyhow::anyhow!("Encrypting aegis database"))?;
            header.params.as_mut().unwrap().tag = ciphertext
                .split_off(ciphertext.len() - 16)
                .try_into()
                .unwrap();
            let db_encrypted = ciphertext;

            *self = Self::Encrypted(AegisEncrypted {
                version: plain_text.version,
                header,
                db: base64::encode(db_encrypted),
            });
        } else {
            // This is an implementation error. Thus, panic is okay.
            panic!("Encrypt can only be called on a plaintext object.")
        }

        Ok(())
    }
}

/// Header of the Encrypted Aegis JSON File
///
/// Contains all necessary information for encrypting / decrypting the vault (db
/// field).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Header {
    #[serde(default)]
    pub slots: Option<Vec<HeaderSlot>>,
    #[serde(default)]
    pub params: Option<HeaderParam>,
}

/// Header Slots
///
/// Containts information to decrypt the master key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeaderSlot {
    // We are not interested in biometric slots at the moment. Thus, we omit these information.
    // However, in the future, authenticator app might be able to lock / unlock the database using
    // fingerprint sensors (see <https://gitlab.gnome.org/World/Authenticator/-/issues/106> for more
    // information). Thus, it might be possible to read also these biometric slots and unlock them
    // with a fingerprint reader used by authenticar. However, it would be ncessary that aegis
    // android app (thus the android system) and authenticator use the same mechanisms to derive
    // keys from biometric input. This has to be checked beforehand.
    //
    // TODO rename should be changed to `rename = 2`. However this does not work yet with serde,
    // see: <https://github.com/serde-rs/serde/issues/745>. This allows decrypting the exported file
    // with the python script provided in the aegis repository. The python script expects an
    // integer but we provide a string. Thus, change the string in header / slots / password
    // slot / `type = "1"` to `type = 1` to use the python script.
    #[serde(rename = "type")]
    pub type_: u32,
    pub uuid: String,
    #[serde(with = "hex::serde")]
    pub key: [u8; 32],
    // First tuple entry is the nonce, the second is the tag.
    pub key_params: HeaderParam,
    n: Option<u32>,
    r: Option<u32>,
    p: Option<u32>,
    #[serde(default, with = "hex::serde")]
    salt: [u8; 32],
}

impl HeaderSlot {
    pub fn n(&self) -> u32 {
        self.n.unwrap_or_else(|| 2_u32.pow(15))
    }

    pub fn r(&self) -> u32 {
        self.r.unwrap_or(8)
    }

    pub fn p(&self) -> u32 {
        self.p.unwrap_or(1)
    }

    pub fn salt(&self) -> &[u8; 32] {
        &self.salt
    }
}

impl Default for HeaderSlot {
    fn default() -> Self {
        let mut rng = rand::thread_rng();
        let mut salt = [0u8; 32];
        rng.fill_bytes(&mut salt);

        Self {
            type_: 1,
            uuid: uuid::Uuid::new_v4().to_string(),
            key: [0u8; 32],
            key_params: HeaderParam::default(),
            n: Some(2_u32.pow(15)),
            r: Some(8),
            p: Some(1),
            salt,
        }
    }
}

/// Parameters to Database Encryption
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeaderParam {
    #[serde(with = "hex::serde")]
    pub nonce: [u8; 12],
    #[serde(with = "hex::serde")]
    pub tag: [u8; 16],
}

impl Default for HeaderParam {
    fn default() -> Self {
        let mut rng = rand::thread_rng();
        let mut nonce = [0u8; 12];
        rng.fill_bytes(&mut nonce);

        Self {
            nonce,
            tag: [0u8; 16],
        }
    }
}

/// Contains All OTP Entries
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Database {
    pub version: u32,
    pub entries: Vec<Item>,
}

impl Default for Database {
    fn default() -> Self {
        Self {
            version: 2,
            entries: std::vec::Vec::new(),
        }
    }
}

/// An OTP Entry
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    #[serde(rename = "type")]
    pub method: OTPMethod,
    // UUID is omitted
    #[serde(rename = "name")]
    pub label: String,
    pub issuer: Option<String>,
    // TODO tags are not imported/exported right now.
    #[serde(rename = "group")]
    pub tags: Option<String>,
    // Note is omitted
    // Icon:
    // TODO: Aegis encodes icons as JPEG's encoded in Base64 with padding. Does authenticator
    // support this?
    // TODO tags are not imported/exported right now.
    #[serde(rename = "icon")]
    pub thumbnail: Option<String>,
    pub info: Detail,
}

impl Item {
    pub fn new(account: &Account) -> Self {
        let provider = account.provider();

        // First, create a detail struct
        let detail = Detail {
            secret: account.token(),
            algorithm: provider.algorithm(),
            digits: provider.digits(),
            // TODO should be none for hotp
            period: Some(provider.period()),
            // TODO should be none for totp
            counter: Some(account.counter()),
        };

        Self {
            method: provider.method(),
            label: account.name(),
            issuer: Some(provider.name()),
            tags: None,
            thumbnail: None,
            info: detail,
        }
    }

    pub fn fix_empty_issuer(&mut self) -> Result<()> {
        if self.issuer.is_none() {
            let mut vals: Vec<&str> = self.label.split("@").collect();
            if vals.len() > 1 {
                self.issuer = vals.pop().map(ToOwned::to_owned);
                self.label = vals.join("@");
            } else {
                anyhow::bail!("Entry {} has an empty issuer", self.label);
            }
        }
        Ok(())
    }
}

/// OTP Entry Details
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Detail {
    pub secret: String,
    #[serde(rename = "algo")]
    pub algorithm: Algorithm,
    pub digits: u32,
    pub period: Option<u32>,
    pub counter: Option<u32>,
}

impl RestorableItem for Item {
    fn account(&self) -> String {
        self.label.clone()
    }

    fn issuer(&self) -> String {
        self.issuer
            .as_ref()
            .map(ToOwned::to_owned)
            .unwrap_or_default()
    }

    fn secret(&self) -> String {
        self.info.secret.clone()
    }

    fn period(&self) -> Option<u32> {
        self.info.period
    }

    fn method(&self) -> OTPMethod {
        self.method
    }

    fn algorithm(&self) -> Algorithm {
        self.info.algorithm
    }

    fn digits(&self) -> Option<u32> {
        Some(self.info.digits)
    }

    fn counter(&self) -> Option<u32> {
        self.info.counter
    }
}

impl Backupable for Aegis {
    const ENCRYPTABLE: bool = true;

    fn identifier() -> String {
        "Aegis".to_string()
    }

    fn title() -> String {
        // Translators: This is for making a backup for the aegis Android app.
        gettext("Aegis")
    }

    fn subtitle() -> String {
        gettext("Into a JSON file containing plain-text or encrypted fields")
    }

    fn backup(model: &ProvidersModel, into: &gtk::gio::File, key: Option<&str>) -> Result<()> {
        // Create structure
        let mut aegis_root = Aegis::default();

        for i in 0..model.n_items() {
            let provider = model.item(i).unwrap().downcast::<Provider>().unwrap();
            let accounts = provider.accounts_model();

            for j in 0..accounts.n_items() {
                let account = accounts.item(j).unwrap().downcast::<Account>().unwrap();
                let otp_item = Item::new(&account);
                aegis_root.add_item(otp_item);
            }
        }

        if let Some(password) = key {
            aegis_root.encrypt(password)?;
        }

        let content = serde_json::ser::to_string_pretty(&aegis_root)?;

        into.replace_contents(
            content.as_bytes(),
            None,
            false,
            gtk::gio::FileCreateFlags::REPLACE_DESTINATION,
            gtk::gio::Cancellable::NONE,
        )?;

        Ok(())
    }
}

impl Restorable for Aegis {
    const ENCRYPTABLE: bool = true;
    const SCANNABLE: bool = false;

    type Item = Item;

    fn identifier() -> String {
        "Aegis".to_string()
    }

    fn title() -> String {
        // Translators: This is for restoring a backup from the aegis Android app.
        gettext("Aegis")
    }

    fn subtitle() -> String {
        gettext("From a JSON file containing plain-text or encrypted fields")
    }

    fn restore_from_data(from: &[u8], key: Option<&str>) -> Result<Vec<Self::Item>> {
        // TODO check whether file / database is encrypted by aegis
        let aegis_root: Aegis = serde_json::de::from_slice(from)?;
        let mut items = Vec::new();

        // Check whether file is encrypted or in plaintext
        match aegis_root {
            Aegis::Plaintext(plain_text) => {
                tracing::info!(
                    "Found unencrypted aegis vault with version {} and database version {}.",
                    plain_text.version,
                    plain_text.db.version
                );

                // Check for correct aegis vault version and correct database version.
                if plain_text.version != 1 {
                    anyhow::bail!(
                        "Aegis vault version expected to be 1. Found {} instead.",
                        plain_text.version
                    );
                // There is no version 0. So this should be okay ...
                } else if plain_text.db.version > 2 {
                    anyhow::bail!(
                        "Aegis database version expected to be 1 or 2. Found {} instead.",
                        plain_text.db.version
                    );
                } else {
                    for mut item in plain_text.db.entries {
                        item.fix_empty_issuer()?;
                        items.push(item);
                    }
                    Ok(items)
                }
            }
            Aegis::Encrypted(encrypted) => {
                tracing::info!(
                    "Found encrypted aegis vault with version {}.",
                    encrypted.version
                );

                // Check for correct aegis vault version and whether a password was supplied.
                if encrypted.version != 1 {
                    anyhow::bail!(
                        "Aegis vault version expected to be 1. Found {} instead.",
                        encrypted.version
                    );
                } else if key.is_none() {
                    anyhow::bail!("Found encrypted aegis database but no password given.");
                }

                // Ciphertext is stored in base64, we have to decode it.
                let mut ciphertext = base64::decode(encrypted.db)
                    .context("Cannot decode (base64) encoded database")?;

                // Add the encryption tag
                ciphertext.append(&mut encrypted.header.params.as_ref().unwrap().tag.into());

                // Find slots with type password and derive the corresponding key. This key is
                // used to decrypt the master key which in turn can be used to
                // decrypt the database.
                let master_keys: Vec<Vec<u8>> = encrypted
                    .header
                    .slots
                    .as_ref()
                    .unwrap()
                    .iter()
                    .filter(|slot| slot.type_ == 1) // We don't handle biometric slots for now
                    .map(|slot| -> Result<Vec<u8>> {
                        tracing::info!("Found possible master key with UUID {}.", slot.uuid);

                        // Create parameters for scrypt function and derive decryption key for
                        // master key
                        //
                        // Somehow, scrypt errors do not implement StdErr and cannot be converted to
                        // anyhow::Error. Should be possible but don't know why it doesn't work.
                        let params = scrypt::Params::new(
                            // TODO log2 for u64 is not stable yet. Change this in the future.
                            (slot.n() as f64).log2() as u8, // Defaults to 15 by aegis
                            slot.r(),                       // Defaults to 8 by aegis
                            slot.p(),                       // Defaults to 1 by aegis
                        )
                        .map_err(|_| anyhow::anyhow!("Invalid scrypt parameters"))?;
                        let mut temp_key: [u8; 32] = [0u8; 32];
                        scrypt::scrypt(
                            key.unwrap().as_bytes(),
                            slot.salt(),
                            &params,
                            &mut temp_key,
                        )
                        .map_err(|_| anyhow::anyhow!("Scrypt key derivation failed"))?;

                        // Now, try to decrypt the master key.
                        let cipher = aes_gcm::Aes256Gcm::new(aes_gcm::Key::from_slice(&temp_key));
                        let mut ciphertext: Vec<u8> = slot.key.to_vec();
                        ciphertext.append(&mut slot.key_params.tag.to_vec());

                        // Here we get the master key. The decrypt function does not return an error
                        // implementing std error. Thus, we have to convert it.
                        cipher
                            .decrypt(
                                aes_gcm::Nonce::from_slice(&slot.key_params.nonce),
                                ciphertext.as_ref(),
                            )
                            .map_err(|_| anyhow::anyhow!("Cannot decrypt master key"))
                    })
                    // Here, we don't want to fail the whole function because one key slot failed to
                    // get the correct master key. Maybe there is another slot we were able to
                    // decrypt.
                    .filter_map(|x| match x {
                        Ok(x) => Some(x),
                        Err(e) => {
                            tracing::error!("Decrypting master key failed: {:?}", e);
                            None
                        }
                    })
                    .collect();

                // Choose the first valid master key. I don't think there are aegis
                // installations with two valid password slots.
                tracing::info!(
                    "Found {} valid password slots / master keys.",
                    master_keys.len()
                );
                let master_key = match master_keys.first() {
                    Some(x) => {
                        tracing::info!("Using only the first valid key slot / master key.");
                        x
                    }
                    None => anyhow::bail!(
                        "Did not find at least one slot with a valid key. Wrong password?"
                    ),
                };

                // Try to decrypt the database with this master key.
                let cipher = aes_gcm::Aes256Gcm::new(aes_gcm::Key::from_slice(master_key));
                let plaintext = cipher
                    .decrypt(
                        aes_gcm::Nonce::from_slice(
                            &encrypted.header.params.as_ref().unwrap().nonce,
                        ),
                        ciphertext.as_ref(),
                    )
                    // Decrypt does not return an error implementing std error, thus we convert it.
                    .map_err(|_| anyhow::anyhow!("Cannot decrypt database"))?;

                // Now, we have the decrypted string. Trying to load it with JSON.
                let db: Database = serde_json::de::from_slice(&plaintext)
                    .context("Deserialize decrypted database failed")?;

                // Check version of the database
                tracing::info!("Found aegis database with version {}.", db.version);
                if encrypted.version > 2 {
                    anyhow::bail!(
                        "Aegis database version expected to be 1 or 2. Found {} instead.",
                        db.version
                    );
                }

                // Return items
                for mut item in db.entries {
                    item.fix_empty_issuer()?;
                    items.push(item);
                }
                Ok(items)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_issuer_from_name() {
        let aegis_data = r#"{
    "version": 1,
    "header": {
        "slots": null,
        "params": null
    },
    "db": {
        "version": 2,
        "entries": [
            {
                "type": "totp",
                "uuid": "01234567-89ab-cdef-0123-456789abcdef",
                "name": "missing-issuer@issuer",
                "issuer": null,
                "icon": null,
                "info": {
                    "secret": "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567",
                    "algo": "SHA1",
                    "digits": 6,
                    "period": 30
                }
            },
            {
                "type": "totp",
                "uuid": "01234567-89ab-cdef-0123-456789abcdef",
                "name": "missing-issuer@domain.com@issuer",
                "issuer": null,
                "icon": null,
                "info": {
                    "secret": "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567",
                    "algo": "SHA1",
                    "digits": 6,
                    "period": 30
                }
            }
        ]
    }
}"#;

        let aegis_items = Aegis::restore_from_data(&aegis_data.as_bytes(), None)
            .expect("Restoring from json should work");

        assert_eq!(aegis_items[0].issuer(), "issuer");
        assert_eq!(aegis_items[0].account(), "missing-issuer");
        assert_eq!(aegis_items[1].issuer(), "issuer");
        assert_eq!(aegis_items[1].account(), "missing-issuer@domain.com");
    }

    #[test]
    fn empty_issuer_failure() {
        let aegis_data = r#"{
    "version": 1,
    "header": {
        "slots": null,
        "params": null
    },
    "db": {
        "version": 2,
        "entries": [
            {
                "type": "totp",
                "uuid": "01234567-89ab-cdef-0123-456789abcdef",
                "name": "cannot-derive-issuer-value",
                "issuer": null,
                "icon": null,
                "info": {
                    "secret": "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567",
                    "algo": "SHA1",
                    "digits": 6,
                    "period": 30
                }
            }
        ]
    }
}"#;

        let result = Aegis::restore_from_data(&aegis_data.as_bytes(), None).unwrap_err();

        assert_eq!(
            format!("{}", result),
            "Entry cannot-derive-issuer-value has an empty issuer"
        );
    }

    #[test]
    fn restore_unencrypted_file() {
        let aegis_data = r#"{
    "version": 1,
    "header": {
        "slots": null,
        "params": null
    },
    "db": {
        "version": 2,
        "entries": [
            {
                "type": "totp",
                "uuid": "01234567-89ab-cdef-0123-456789abcdef",
                "name": "Bob",
                "issuer": "Google",
                "icon": null,
                "info": {
                    "secret": "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567",
                    "algo": "SHA1",
                    "digits": 6,
                    "period": 30
                }
            },
            {
                "type": "hotp",
                "uuid": "03e572f2-8ebd-44b0-a57e-e958af74815d",
                "name": "Benjamin",
                "issuer": "Air Canada",
                "icon": null,
                "info": {
                    "secret": "KUVJJOM753IHTNDSZVCNKL7GII",
                    "algo": "SHA256",
                    "digits": 7,
                    "counter": 50
                }
            },
            {
                "type": "steam",
                "uuid": "5b11ae3b-6fc3-4d46-8ca7-cf0aea7de920",
                "name": "Sophia",
                "issuer": "Boeing",
                "icon": null,
                "info": {
                    "secret": "JRZCL47CMXVOQMNPZR2F7J4RGI",
                    "algo": "SHA1",
                    "digits": 5,
                    "period": 30
                }
            }
        ]
    }
}"#;

        let aegis_items = Aegis::restore_from_data(&aegis_data.as_bytes(), None)
            .expect("Restoring from json should work");

        assert_eq!(aegis_items[0].account(), "Bob");
        assert_eq!(aegis_items[0].issuer(), "Google");
        assert_eq!(aegis_items[0].secret(), "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567");
        assert_eq!(aegis_items[0].period(), Some(30));
        assert_eq!(aegis_items[0].algorithm(), Algorithm::SHA1);
        assert_eq!(aegis_items[0].digits(), Some(6));
        assert_eq!(aegis_items[0].counter(), None);
        assert_eq!(aegis_items[0].method(), OTPMethod::TOTP);

        assert_eq!(aegis_items[1].account(), "Benjamin");
        assert_eq!(aegis_items[1].issuer(), "Air Canada");
        assert_eq!(aegis_items[1].secret(), "KUVJJOM753IHTNDSZVCNKL7GII");
        assert_eq!(aegis_items[1].period(), None);
        assert_eq!(aegis_items[1].algorithm(), Algorithm::SHA256);
        assert_eq!(aegis_items[1].digits(), Some(7));
        assert_eq!(aegis_items[1].counter(), Some(50));
        assert_eq!(aegis_items[1].method(), OTPMethod::HOTP);

        assert_eq!(aegis_items[2].account(), "Sophia");
        assert_eq!(aegis_items[2].issuer(), "Boeing");
        assert_eq!(aegis_items[2].secret(), "JRZCL47CMXVOQMNPZR2F7J4RGI");
        assert_eq!(aegis_items[2].period(), Some(30));
        assert_eq!(aegis_items[2].algorithm(), Algorithm::SHA1);
        assert_eq!(aegis_items[2].digits(), Some(5));
        assert_eq!(aegis_items[2].counter(), None);
        assert_eq!(aegis_items[2].method(), OTPMethod::Steam);
    }

    #[test]
    fn deserialize_encrypted() {
        let aegis_data = r#"{
            "version": 1,
            "header": {
                "slots": [
                    {
                        "type": 1,
                        "uuid": "an-uuid",
                        "key": "491d44550430ba248986b904b8cffd3a6c5755d176ac877bd11b82c934225017",
                        "key_params": {
                            "nonce": "095fd13dee336fa56b4634ff",
                            "tag": "5db2470edf2d12f82a89ae7f48ccd50c"
                        },
                        "n": 64604640,
                        "r": 10,
                        "p": 12,
                        "salt": "27ea9ae53fa2f08a8dcd201615a8229422647b3058f9f36b08f9457e62888be1",
                        "repaired": true
                    },
                    {
                        "type": 2,
                        "uuid": "some-uuid",
                        "key": "491d44550430ba248986b904b8cffd3a6c5755d176ac877bd11b82c934225017",
                        "key_params": {
                            "nonce": "095fd13dee336fa56b4634ff",
                            "tag": "5db2470edf2d12f82a89ae7f48ccd50c"
                        }
                    }
                ],
                "params": {
                    "nonce": "095fd13dee336fa56b4634ff",
                    "tag": "5db2470edf2d12f82a89ae7f48ccd50c"
                }
            },
            "db": "the encrypted DB"
        }"#;

        let data: Result<Aegis, _> = serde_json::de::from_slice(&aegis_data.as_bytes());
        assert!(data.is_ok());
    }

    #[test]
    fn restore_encrypted_file() {
        // See <https://github.com/beemdevelopment/Aegis/blob/master/app/src/test/resources/com/beemdevelopment/aegis/importers/aegis_encrypted.json>
        // for this example file.
        let aegis_data = r#"{
    "version": 1,
    "header": {
        "slots": [
            {
                "type": 1,
                "uuid": "a8325752-c1be-458a-9b3e-5e0a8154d9ec",
                "key": "491d44550430ba248986b904b8cffd3a6c5755d176ac877bd11b82c934225017",
                "key_params": {
                    "nonce": "e9705513ba4951fa7a0608d2",
                    "tag": "931237af257b83c693ddb8f9a7eddaf0"
                },
                "n": 32768,
                "r": 8,
                "p": 1,
                "salt": "27ea9ae53fa2f08a8dcd201615a8229422647b3058f9f36b08f9457e62888be1",
                "repaired": true
            }
        ],
        "params": {
            "nonce": "095fd13dee336fa56b4634ff",
            "tag": "5db2470edf2d12f82a89ae7f48ccd50c"
        }
    },
    "db": "RtGfUrZ01nzRnvHjPJGyWjfa6shQ7NYwa491CgAWNBM8OeGZVIHhnDAVlVWNlSoq2V097p5Yq5m+SFl5g9nBBBQBNePQnj6CCvu1NfNtoA6R3hyp77gd+e+O2MRnOGH1Z1laV2Tl6p3q8IUHWgAJ36LbUxiCXmfh7bWm198uA4bgLwrEmo04MrqeYXggLuXrJrp6dUJQFD72dgoPbHijlSycY5GLel3ZbAXRsUHszd+xdywpj7\/TYa4OYFel0M0QcCpsKA1LRQz365X9OXPJdTsmVyR4dJ6x5RIVeh39lAYKUf7T4w7BLC8taST5m4J\/VXDueKbvg8R13bNWF0aRHUgeuI9BNzMZINJlzKFKNRknTaJ\/1kEUU0sLkgcaVkX\/DVTGG+pWi5MHijicrK0i4LHN3CUwV2\/\/ZNJCGXM5ErsKMOnJfma52gMdifPiXU317Klvc5oOZFYGnhbhJ2WtPIuqjdvnfuLat2JxA7Xx3LqquRWGL2113yjzVzGBDCVY6iIdedBEgH8CGD826\/3R3m6dR5sfSggQ2SbtQA\/DZNhLSNSU+bfNScVQvUWfR2Lf7Q\/4FR\/xATAQJ9IIBeL+w2ErLUPjURocFXup5YOBHxFdDjZ2FqhbAq4h3Zn\/BJ57xUcYEA+YtP5uOP2lQwUh\/0vFWizDVotzraO8tZiBZBsODyb69eJrXNwFbIjeUczY6wrJs1+676IilbCsmtoYvWEpUZF4hIi7TYAD+nyXX\/olrkog9omWZk8R7hJ9KRDfckXEc\/XSzWhk3Kmfa7pRNh9wYZsaR7VPZGZebQMuUKfRRci2qMsZOJvQsDBJvVze0xW9SqiySDgGyRX\/DwzuaZEGZZriaLf6ox7LwY2Qi6QpYOYbAaEaXAesCR1DPxFfGKsUHVjF8hKA6ZBXDXdqM3Y+14naIOH9S7UzYn32botoVLOykSjnW6z6M0ZPkz3dwowMJiVQcyD7p+9p4J6f1S81pFS7DP+jF+PTyC3c3q\/dwFhNdoG6iV9eQEAxjUi6MpzvFRsk9RsLcQqYgzJGmRjYeXlKH8k8tTu1A4puo6w3Daz8hZz9NafMgMsuqY0oKVLgdNqFz8yVMsxYfBW\/oW56SuQyyVWyxXjXmbk1vpYCTL5kXvIZWoTmBRRDb0ay5S\/dlD6z\/WR45\/C4AwcCE9m4Yf3zisRNa7AqWLVgkmJxFdfJxjiuPtUIK79s+lIJkyRENEqkvm809qIxDhkQzY8zcCt4oXCEbJUfSG4awBs1VvilJIwe6qi0bNtqXtAb5TctgxTh29A9oGlsRG4o8sHqA1mtjp5QiLWp5Hh6rOH95W6+fnBiOW+Iw0evBTduroWvx37HBTktJz79zGe0l3c0Y6VmiFvB7knmT2CrgP7woRkxGbXxdE9zMPQJM9ursD538MVDdD\/0tdkxHxilt47f1DPo2CKUWU8Q1KMm1zLXfVO8BbGUWIv4YeDKHfMUL\/HcStv5VJY+LbnOEjzGT4e1\/avSQmqBL4G9XNkYmyMhC8tlLQcmMMH4bNfPOO3vi5Pb5E7XveSgxlOHs4F0+nqxnFOAu8494MEtx6u5+B7d8LI\/DhEO5zTDwE+THiKej6vCsFxTZ519rm67HycOwRR4LKrwfDeUEK3X1PzryOD5zcv3PMcSBgZ8EWvTfZ9ygKP8BmRQRpydTbSt8Hj5fTUuajADCP0Ggw+6G7n+5FhExJNd+o9D8d4KgLPOe08M8InW7pLB389TWtSo4v3VNjcmmJNQ26wlPkhO\/xBU1URFR0fXU3eCO+w++IMt\/fOSqSpNF9bWElfWHIQ23ntxVke\/hR9j\/GG3tHGxYS5pL42sJF\/Re\/UlUJTGSQP6up2xVYs6gncQ0zACDOPjLQmQzYhz\/hr8S6EjYfK++yLZmRTjEI7xT9u\/B5YLyOQCYVTaF\/pDEegjsehXM3qJBfsA+XY7F9TRsmM\/MSVaPDkdIJ7zvL9xtaF6bXdZoZ6po3ml8uu41pSkNmMKgyEy5E0UQUTWMPLC8drUoQ\/KWQnVIN6HUXGBjYy6aax\/LYZaBcbZi97FHK0h+wsx3WN\/uQozNkQjwGYE8fwYxRYh1RaFi5PkiCM505ib7e82Yuts0l+cBb6nG1IruDplg9BD\/G9w4vVDePEikhcPyY\/p7AZ4i7u\/bL2YKlbE3HyJa+7dkbWJgGidtRZgu+Fdl2T\/rrRJ4+lVaKPVKGKT7ItZdIeitIYUdRxCzrOf1ItZCC8BWa4PElDAjj2yDNmMYRpXJBe3gQHWs\/H5SZgFuwsfCu23uzNRQYib8SuwIJQDvPiXo7m4oIySO8VyvemcExlbXSlbZbvwVxYavTVfcUpAXI6qlsg2jjk+JZahfKrWNC5COZPdVjdAXCoiKU+HBPmEFCwQv\/7zlSBEiI2piyqd+MPwnP63RdGO+oXYid6hn4Nm8kcOhtRyvYm95p66jzGlEugsfxJCED7MTh3XShqa2tt4lFG25icllzTvIJboRkz5oIB4dZVS9+q2TgGUoX7UCpobD8WkHo\/y0cpTuZr8vzXqx2fObxzPNoVgxJmp9E06G2bhMVHPpT17xbfq\/KhJJn7k1S0sfXPG+SmYlX4U7zNSe1M7JXtLf3uVOLz7Ccjp3yvcdq8nRmVym3Zwsz+vv57FA2A0dy3Db97ypJa9HGaxnnYIZHHzep0gJCeeIKE9L32zGCoUg+cPu9B2lPEgIr64iGiuvKSRwNQpOBktM6qqjQntE0Me6mh426irFQ\/3tcfH9a4lZEwwuU1X+lUBUWQp3n5Ej4BSJEs8E6H0EjBvyk69q3qjy5yi7ROVRis6y6S1v4er77RHQUf3phK5354VJHrp9pR926t5qngH5RVF4eljwtXDs3MkejADJ6stBHa\/w7FcbUClO8U+S4Bidxb3mZCiZkUVTpbzvBfYAiQvAfdkMa49o3a5DXKsbXyUPrmr6fWRfM1fS0Ehp0lUv6BDj0yR13CLMpKDU4GfDrl8UEvwh7gwtBRkuaBFzyMtd3NeE7kIGf9vFs6MEl2dmMDFSDid7MdVSDVTlhaAtp+zsRejKW3OQr5n051FzkUsIFGty9AWOkwjZCbstHYCOtyJnsnXP1i9lRDFBgPpFgmDD+bzzg0g9AOAxzqTiLF7bb1jejfe5qVr5V9+7zLpwRLiYaLkNOmpsqvNMuYVwdqTp6nyoougdgBlvve3EG0k09sFKi2Ep9lq+QkS7zGre2jJDrqgdC08+V4PXHYkP3V3Zjgn1x6RfQ2PE+2zvk1GGEgzcNww3byoYw0Ra5qS5yftMy\/2WahbA8fjUYvtmksFH8VjN3yasZt3sdQLWtv8qXxZscy+pCyjTdyxW+ddFnrWuqMIV3jbGMvngq6dL\/n5+DumjbA1gmBJVOpmyEsc1iwHDS36cNnyi1htGFO\/6\/Va4YPYK7dG6LY387UoBUU9Q9ijrBrSGpzPWYmXBLZ8e1MMPfHIN1WsaTgYO9leg3MAJTjQFTFrQ5dguYpWhlm2sWJT45jrda4uWqduB+aQLzYRWhEDBFzPV3ZgIe0SB+7h04Vm0Pu\/LDRvqaolpZ86CEm+zgjBOKeEGFwzTXxH\/5pBoca1bZ6wvsbVZxJNBeH8\/w=="
}"#;

        let aegis_items = Aegis::restore_from_data(&aegis_data.as_bytes(), Some("test"))
            .expect("Restoring from encrypted json should work");

        assert_eq!(aegis_items[0].account(), "Mason");
        assert_eq!(aegis_items[0].issuer(), "Deno");
        assert_eq!(aegis_items[0].secret(), "4SJHB4GSD43FZBAI7C2HLRJGPQ");
        assert_eq!(aegis_items[0].period(), Some(30));
        assert_eq!(aegis_items[0].algorithm(), Algorithm::SHA1);
        assert_eq!(aegis_items[0].digits(), Some(6));
        assert_eq!(aegis_items[0].counter(), None);
        assert_eq!(aegis_items[0].method(), OTPMethod::TOTP);

        assert_eq!(aegis_items[3].account(), "James");
        assert_eq!(aegis_items[3].issuer(), "Issuu");
        assert_eq!(aegis_items[3].secret(), "YOOMIXWS5GN6RTBPUFFWKTW5M4");
        assert_eq!(aegis_items[3].period(), None);
        assert_eq!(aegis_items[3].algorithm(), Algorithm::SHA1);
        assert_eq!(aegis_items[3].digits(), Some(6));
        assert_eq!(aegis_items[3].counter(), Some(1));
        assert_eq!(aegis_items[3].method(), OTPMethod::HOTP);

        assert_eq!(aegis_items[6].account(), "Sophia");
        assert_eq!(aegis_items[6].issuer(), "Boeing");
        assert_eq!(aegis_items[6].secret(), "JRZCL47CMXVOQMNPZR2F7J4RGI");
        assert_eq!(aegis_items[6].period(), Some(30));
        assert_eq!(aegis_items[6].algorithm(), Algorithm::SHA1);
        assert_eq!(aegis_items[6].digits(), Some(5));
        assert_eq!(aegis_items[6].counter(), None);
        assert_eq!(aegis_items[6].method(), OTPMethod::Steam);
    }

    // TODO: add tests for importing
}

use crate::models::{Algorithm, OTPMethod, OTPUri};
use percent_encoding::percent_decode;
use prost::{Enumeration, Message};
use std::borrow::Cow;
use url::Url;

#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone)]
pub struct OTPMigrationUri {
    children: Vec<OTPUri>,
}

impl TryFrom<Url> for OTPMigrationUri {
    type Error = anyhow::Error;
    fn try_from(url: Url) -> Result<Self, Self::Error> {
        if url.scheme() != "otpauth-migration" {
            anyhow::bail!("Invalid OTP migration uri format, expected uri protocol to be otpauth-migration, got {}", url.scheme());
        }

        if let Some(host) = url.host_str() {
            if host != "offline" {
                anyhow::bail!("Invalid OTP migration uri format, expected uri host to be offline, got {host}");
            }
        } else {
            anyhow::bail!("Invalid OTP migration uri format, expected uri host to be offline, got nothing");
        }

        let data = url
            .query_pairs()
            .fold(None, |folded, (key, value)| folded.or_else(|| match key.into_owned().as_str() {
                "data" => {
                    let bytes = value.into_owned().into_bytes();
                    let decoded = percent_decode(&*bytes);
                    let decoded = match base64::decode(&*Cow::from(decoded)) {
                        Ok(decoded) => decoded,
                        Err(_) => return None,
                    };
                    Some(match protobuf::MigrationPayload::decode(&*decoded) {
                        Ok(decoded) => decoded,
                        Err(_) => return None,
                    })
                },
                _ => None,
            }))
            .map(|data| protobuf::MigrationPayload {
                // Filter out invalid OTP URIs
                otp_parameters: {
                    let otp_parameters_len = data.otp_parameters.len();

                    let mut otp_parameters = data.otp_parameters
                        .into_iter()
                        .fold(
                            Vec::with_capacity(otp_parameters_len),
                            |mut folded, otp_parameters| {
                                if otp_parameters.algorithm() == protobuf::migration_payload::Algorithm::ALGO_INVALID {
                                    return folded;
                                }

                                if otp_parameters.r#type() == protobuf::migration_payload::OtpType::OTP_INVALID {
                                    return folded;
                                }

                                if !folded.contains(&otp_parameters) {
                                    folded.push(otp_parameters);
                                }

                                folded
                            });

                    otp_parameters.shrink_to_fit();

                    otp_parameters
                },
                ..data
            });

        let data = if let Some(data) = data {
            data
        } else {
            anyhow::bail!("Invalid OTP migration uri format, expected a data query parameter");
        };

        let children_len = data.otp_parameters.len();

        Ok(OTPMigrationUri {
            children: {
                let mut otp_parameters = data.otp_parameters
                    .into_iter()
                    .fold(Vec::with_capacity(children_len), |mut folded, otp| {
                        folded.push(OTPUri {
                            algorithm: match otp.algorithm() {
                                protobuf::migration_payload::Algorithm::ALGO_SHA1 => Algorithm::SHA1,
                                _ => unreachable!(),
                            },
                            digits: match otp.r#type() {
                                protobuf::migration_payload::OtpType::OTP_HOTP => Some(otp.digits as u32),
                                _ => None,
                            },
                            method: match otp.r#type() {
                                protobuf::migration_payload::OtpType::OTP_HOTP => OTPMethod::HOTP,
                                protobuf::migration_payload::OtpType::OTP_TOTP => OTPMethod::TOTP,
                                _ => unreachable!(),
                            },
                            secret: {
                                let secret = &*otp.secret;

                                let mut buffer = [0; 128];

                                match binascii::b32encode(secret, &mut buffer) {
                                    Ok(_) => (),
                                    Err(_) => return folded,
                                }

                                let buffer = buffer.to_vec();

                                let string = match String::from_utf8(buffer) {
                                    Ok(string) => string,
                                    Err(_) => return folded,
                                };

                                string.trim_end_matches(|c| c == '\0' || c == '=').to_owned()
                            },
                            label: otp.name,
                            issuer: otp.issuer,
                            period: None,
                            counter: Some(otp.counter as u32),
                        });
                        folded
                    });

                otp_parameters.shrink_to_fit();

                otp_parameters
            },
        })
    }
}

mod into_iterator {
    use super::*;

    impl IntoIterator for OTPMigrationUri {
        type Item = OTPUri;
        type IntoIter = IntoIter;

        fn into_iter(self) -> Self::IntoIter {
            IntoIter {
                children: self.children.into_iter().rev().collect(),
            }
        }
    }

    #[allow(clippy::upper_case_acronyms)]
    #[derive(Debug, Clone)]
    pub struct IntoIter {
        children: Vec<OTPUri>,
    }

    impl IntoIter {
        pub fn index(&self) -> usize {
            self.children.capacity() - self.children.len()
        }

        pub fn len(&self) -> usize {
            self.children.capacity()
        }
    }

    impl Iterator for IntoIter {
        type Item = OTPUri;

        fn next(&mut self) -> Option<Self::Item> {
            self.children.pop()
        }
    }
}

#[allow(non_camel_case_types)]
mod protobuf {
    use super::*;

    #[derive(Clone, Message)]
    pub struct MigrationPayload {
        #[prost(message, repeated)]
        pub otp_parameters: Vec<migration_payload::OtpParameters>,
        #[prost(int32)]
        pub version: i32,
        #[prost(int32)]
        pub batch_size: i32,
        #[prost(int32)]
        pub batch_index: i32,
        #[prost(int32)]
        pub batch_id: i32,
    }

    pub mod migration_payload {
        use super::*;

        #[derive(Debug, Copy, Clone, PartialEq, Eq, Enumeration)]
        pub enum Algorithm {
            ALGO_INVALID = 0,
            ALGO_SHA1 = 1,
        }

        #[derive(Debug, Copy, Clone, PartialEq, Eq, Enumeration)]
        pub enum OtpType {
            OTP_INVALID = 0,
            OTP_HOTP = 1,
            OTP_TOTP = 2,
        }

        #[derive(Clone, PartialEq, Eq, Message)]
        pub struct OtpParameters {
            #[prost(bytes)]
            pub secret: Vec<u8>,
            #[prost(string)]
            pub name: String,
            #[prost(string)]
            pub issuer: String,
            #[prost(enumeration = "Algorithm")]
            pub algorithm: i32,
            #[prost(int32)]
            pub digits: i32,
            #[prost(enumeration = "OtpType")]
            pub r#type: i32,
            #[prost(int64)]
            pub counter: i64,
        }
    }
}


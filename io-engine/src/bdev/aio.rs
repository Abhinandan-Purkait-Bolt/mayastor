use std::{
    collections::HashMap,
    convert::TryFrom,
    ffi::CString,
    fmt::{Debug, Formatter},
};

use async_trait::async_trait;
use futures::channel::oneshot;
use nix::errno::Errno;
use snafu::ResultExt;
use url::Url;

use spdk_rs::libspdk::{bdev_aio_delete, create_aio_bdev, spdk_accel_crypto_key_create_param, spdk_create_crypto_bdev};

use crate::{
    bdev::{dev::reject_unknown_parameters, util::uri, CreateDestroy, GetName},
    bdev_api::{self, BdevError},
    core::{UntypedBdev, VerboseError},
    ffihelper::{cb_arg, done_errno_cb, ErrnoResult}, pool_backend::Encryption,
};

pub(super) struct Aio {
    name: String,
    alias: String,
    blk_size: u32,
    uuid: Option<uuid::Uuid>,
}

impl Debug for Aio {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Aio '{}'", self.name)
    }
}

/// Convert a URI to an Aio "object"
impl TryFrom<&Url> for Aio {
    type Error = BdevError;

    fn try_from(url: &Url) -> Result<Self, Self::Error> {
        let segments = uri::segments(url);

        if segments.is_empty() {
            return Err(BdevError::InvalidUri {
                uri: url.to_string(),
                message: String::from("no path segments"),
            });
        }

        let mut parameters: HashMap<String, String> =
            url.query_pairs().into_owned().collect();

        let blk_size: u32 = match parameters.remove("blk_size") {
            Some(value) => {
                value.parse().context(bdev_api::IntParamParseFailed {
                    uri: url.to_string(),
                    parameter: String::from("blk_size"),
                    value: value.clone(),
                })?
            }
            None => 512,
        };

        let uuid = uri::uuid(parameters.remove("uuid")).context(
            bdev_api::UuidParamParseFailed {
                uri: url.to_string(),
            },
        )?;

        reject_unknown_parameters(url, parameters)?;

        Ok(Aio {
            name: url.path().into(),
            alias: url.to_string(),
            blk_size,
            uuid,
        })
    }
}

impl GetName for Aio {
    fn get_name(&self, crypto: bool) -> String {
        if crypto {
            self.name.clone() + "_crypto"
        } else {
            self.name.clone()
        }
    }
}
fn create_crypto_bdev(name: String, base_bdev_name: String, encrypt_param: Encryption) {
    // let cipher = CString::new("AES_XTS").expect("Failed to create cipher CString");
    // let hex_key = CString::new("00112233445566778899aabbccddeeff").expect("Failed to create hex_key CString");
    // let hex_key2 = CString::new("ffeeddccbbaa99887766554433221100").expect("Failed to create hex_key2 CString");
    // let key_name = CString::new("ut_key").expect("Failed to create key_name CString");
    let cipher: CString = CString::new(encrypt_param.cipher).unwrap();
    let hex_key: CString = CString::new(encrypt_param.hex_key1).unwrap();
    let hex_key2: CString = CString::new(encrypt_param.hex_key2).unwrap();
    let key_name: CString = CString::new(encrypt_param.key_name).unwrap();

    let mut key_params = spdk_accel_crypto_key_create_param {
        cipher: cipher.as_ptr() as *mut i8,
        hex_key: hex_key.as_ptr() as *mut i8,
        hex_key2: hex_key2.as_ptr() as *mut i8,
        key_name: key_name.as_ptr() as *mut i8,
        tweak_mode: std::ptr::null_mut(),
    };
    info!("create_crypto_bdev: name: {:?}, base_bdev_name: {:?}", name, base_bdev_name);
    let cname = CString::new(name).unwrap();
    let cbase_bdev_name = CString::new(base_bdev_name).unwrap();
    let errno = unsafe {
        spdk_create_crypto_bdev(cname.as_ptr() as *mut i8, cbase_bdev_name.as_ptr() as *mut i8, &mut key_params as *mut _)
    };
    info!("create_crypto_bdev: {:?}", errno);
}
#[async_trait(?Send)]
impl CreateDestroy for Aio {
    type Error = BdevError;

    /// Create an AIO bdev
    async fn create(&self, _encrypt: Option<Encryption>) -> Result<String, Self::Error> {
        if UntypedBdev::lookup_by_name(&self.name).is_some() {
            return Err(BdevError::BdevExists {
                name: self.get_name(_encrypt.is_some()),
            });
        }
        let encrypt = Some(Encryption {
            cipher: "AES_XTS".to_string(),
            hex_key1: "00112233445566778899aabbccddeeff".to_string(),
            hex_key2: "ffeeddccbbaa99887766554433221100".to_string(),
            key_name: "ut_key".to_string(),
        });
        
        let cname = CString::new(self.get_name(false)).unwrap();

        let errno = unsafe {
            create_aio_bdev(
                cname.as_ptr(),
                cname.as_ptr(),
                self.blk_size,
                false,
            )
        };

        if errno != 0 {
            let err = BdevError::CreateBdevFailed {
                source: Errno::from_i32(errno.abs()),
                name: self.get_name(false),
            };

            error!("{:?} error: {}", self, err.verbose());

            return Err(err);
        }
        if let Some(encrypt_param) = encrypt {
            let crypto_name = self.get_name(true);
            create_crypto_bdev(crypto_name.clone(), self.name.clone(), encrypt_param);
            if let Some(mut bdev) = UntypedBdev::lookup_by_name(&crypto_name) {
                if let Some(uuid) = self.uuid {
                    unsafe { bdev.set_raw_uuid(uuid.into()) };
                }
                if !bdev.add_alias(&crypto_name) {
                    warn!("{:?}: failed to add alias '{}'", self, crypto_name);
                }

                return Ok(crypto_name);
            }
            Err(BdevError::BdevNotFound {
                name: self.get_name(false),
            })
        } else {
            if let Some(mut bdev) = UntypedBdev::lookup_by_name(&self.name) {
                if let Some(uuid) = self.uuid {
                    unsafe { bdev.set_raw_uuid(uuid.into()) };
                }
                if !bdev.add_alias(&self.alias) {
                    warn!("{:?}: failed to add alias '{}'", self, self.alias);
                }

                return Ok(self.get_name(false));
            }
            Err(BdevError::BdevNotFound {
                name: self.get_name(false),
            })
        }
    }

    /// Destroy the given AIO bdev
    async fn destroy(self: Box<Self>) -> Result<(), Self::Error> {
        debug!("{:?}: deleting", self);

        match UntypedBdev::lookup_by_name(&self.name) {
            Some(mut bdev) => {
                bdev.remove_alias(&self.alias);
                let (sender, receiver) = oneshot::channel::<ErrnoResult<()>>();
                unsafe {
                    bdev_aio_delete(
                        (*bdev.unsafe_inner_ptr()).name,
                        Some(done_errno_cb),
                        cb_arg(sender),
                    );
                }
                receiver
                    .await
                    .context(bdev_api::BdevCommandCanceled {
                        name: self.get_name(false),
                    })?
                    .context(bdev_api::DestroyBdevFailed {
                        name: self.get_name(false),
                    })
            }
            None => Err(BdevError::BdevNotFound {
                name: self.get_name(false),
            }),
        }
    }
}

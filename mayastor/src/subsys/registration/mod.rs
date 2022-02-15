//! Message bus connecting mayastor to the control plane components.
//!
//! It is designed to make sending events to control plane easy in the future.
//!
//! A Registration subsystem is used to keep moac in the loop
//! about the lifecycle of mayastor instances.

pub mod registration_grpc;

use crate::core::MayastorEnvironment;
use registration_grpc::Registration;
use spdk_rs::libspdk::{
    spdk_add_subsystem,
    spdk_subsystem,
    spdk_subsystem_fini_next,
    spdk_subsystem_init_next,
};

// wrapper around our Registration subsystem used for registration
pub struct RegistrationSubsystem(*mut spdk_subsystem);

impl Default for RegistrationSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistrationSubsystem {
    /// initialise a new subsystem that handles the control plane
    /// registration process
    extern "C" fn init() {
        unsafe { spdk_subsystem_init_next(0) }
    }

    extern "C" fn fini() {
        debug!("mayastor registration subsystem fini");
        let args = MayastorEnvironment::global_or_default();
        if args.grpc_endpoint.is_some() {
            if let Some(registration) = Registration::get() {
                registration.fini();
            }
        }
        unsafe { spdk_subsystem_fini_next() }
    }

    fn new() -> Self {
        info!("creating Mayastor registration subsystem...");
        let mut ss = Box::new(spdk_subsystem::default());
        ss.name = b"mayastor_mbus\x00" as *const u8 as *const libc::c_char;
        ss.init = Some(Self::init);
        ss.fini = Some(Self::fini);
        ss.write_config_json = None;
        Self(Box::into_raw(ss))
    }

    /// register the subsystem with spdk
    pub(super) fn register() {
        unsafe { spdk_add_subsystem(RegistrationSubsystem::new().0) }
    }
}

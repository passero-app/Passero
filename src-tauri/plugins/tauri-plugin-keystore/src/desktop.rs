use serde::de::DeserializeOwned;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

use crate::models::*;

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<Keystore<R>> {
    Ok(Keystore(app.clone()))
}

/// Access to the keystore APIs.
pub struct Keystore<R: Runtime>(AppHandle<R>);

impl<R: Runtime> Keystore<R> {
    pub fn store(&self, payload: StoreRequest) -> crate::Result<()> {
        let entry = keyring::Entry::new(&payload.service, &payload.user).unwrap();
        entry.set_password(&payload.value).unwrap();
        Ok(())
    }

    pub fn retrieve(&self, payload: RetrieveRequest) -> crate::Result<RetrieveResponse> {
        let entry = keyring::Entry::new(&payload.service, &payload.user).unwrap();
        Ok(RetrieveResponse {
            value: entry.get_password().ok(),
        })
    }

    // TODO: remove unwrap() calls
    pub fn remove(&self, payload: RemoveRequest) -> crate::Result<()> {
        let entry = keyring::Entry::new(&payload.service, &payload.user).unwrap();
        entry.delete_credential().unwrap();
        Ok(())
    }
}

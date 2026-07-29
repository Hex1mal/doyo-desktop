use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(rename = "minApiVersion")]
    pub min_api_version: u32,
    pub author: String,
    pub description: String,
    pub permissions: Vec<String>,
    pub main: String,
}

#[cfg(feature = "plugin-runtime")]
mod runtime_impl {
    use super::*;
    use rquickjs::{Context, Runtime};

    pub struct PluginRuntime {
        pub runtime: Runtime,
        pub context: Context,
        pub plugin_id: String,
        pub granted_permissions: Vec<String>,
    }

    pub struct PluginManager {
        plugins: Mutex<HashMap<String, PluginRuntime>>,
    }

    impl PluginManager {
        pub fn new() -> Self {
            Self {
                plugins: Mutex::new(HashMap::new()),
            }
        }

        pub fn load_plugin(
            &self,
            manifest: &PluginManifest,
            source: &str,
        ) -> Result<()> {
            let runtime = Runtime::new().map_err(|e| {
                Error::InvalidOperation(format!("Failed to create JS runtime: {}", e))
            })?;

            let context = Context::full(&runtime).map_err(|e| {
                Error::InvalidOperation(format!("Failed to create JS context: {}", e))
            })?;

            context.with(|ctx| {
                let global = ctx.globals();

                let api_obj = rquickjs::Object::new(ctx.clone()).map_err(|e| {
                    Error::InvalidOperation(format!("Failed to create API object: {}", e))
                })?;

                api_obj.set("version", 1).map_err(|e| {
                    Error::InvalidOperation(format!("Failed to set version: {}", e))
                })?;

                api_obj.set("platform", std::env::consts::OS).ok();

                // Set up __app global with plugin API
                setup_log_api(ctx, &api_obj, &manifest.id)?;
                setup_metadata_api(ctx, &api_obj)?;
                setup_ui_api(ctx, &api_obj, manifest)?;
                setup_nodes_api(ctx, &api_obj, manifest)?;
                setup_storage_api(ctx, &api_obj)?;
                setup_events_api(ctx, &api_obj)?;

                global.set("__app", api_obj).map_err(|e| {
                    Error::InvalidOperation(format!("Failed to set __app global: {}", e))
                })?;

                ctx.eval::<(), _>(source).map_err(|e| {
                    Error::InvalidOperation(format!("Plugin script error: {}", e))
                })?;

                Ok::<_, Error>(())
            })?;

            self.plugins
                .lock()
                .map_err(|e| Error::InvalidOperation(format!("Lock error: {}", e)))?
                .insert(
                    manifest.id.clone(),
                    PluginRuntime {
                        runtime,
                        context,
                        plugin_id: manifest.id.clone(),
                        granted_permissions: manifest.permissions.clone(),
                    },
                );

            Ok(())
        }

        pub fn unload_plugin(&self, plugin_id: &str) -> Result<()> {
            self.plugins
                .lock()
                .map_err(|e| Error::InvalidOperation(format!("Lock error: {}", e)))?
                .remove(plugin_id);
            Ok(())
        }

        pub fn has_permission(&self, plugin_id: &str, permission: &str) -> bool {
            self.plugins
                .lock()
                .ok()
                .and_then(|plugins| {
                    plugins
                        .get(plugin_id)
                        .map(|p| p.granted_permissions.contains(&permission.to_string()))
                })
                .unwrap_or(false)
        }

        pub fn plugin_count(&self) -> usize {
            self.plugins.lock().map(|p| p.len()).unwrap_or(0)
        }
    }

    fn setup_log_api(
        ctx: &rquickjs::Ctx<'_>,
        api_obj: &rquickjs::Object<'_>,
        plugin_id: &str,
    ) -> Result<()> {
        let log_obj = rquickjs::Object::new(ctx.clone()).unwrap();
        let pid = plugin_id.to_string();

        log_obj
            .set(
                "info",
                rquickjs::Function::new(ctx.clone(), move |msg: String| {
                    log::info!("[plugin:{}] {}", pid, msg);
                }),
            )
            .ok();

        log_obj
            .set(
                "error",
                rquickjs::Function::new(ctx.clone(), move |msg: String| {
                    log::error!("[plugin:{}] {}", pid, msg);
                }),
            )
            .ok();

        log_obj
            .set(
                "warn",
                rquickjs::Function::new(ctx.clone(), move |msg: String| {
                    log::warn!("[plugin:{}] {}", pid, msg);
                }),
            )
            .ok();

        log_obj
            .set(
                "debug",
                rquickjs::Function::new(ctx.clone(), move |msg: String| {
                    log::debug!("[plugin:{}] {}", pid, msg);
                }),
            )
            .ok();

        api_obj.set("log", log_obj).ok();
        Ok(())
    }

    fn setup_metadata_api(
        ctx: &rquickjs::Ctx<'_>,
        api_obj: &rquickjs::Object<'_>,
    ) -> Result<()> {
        let metadata_obj = rquickjs::Object::new(ctx.clone()).unwrap();
        metadata_obj.set("appVersion", env!("CARGO_PKG_VERSION")).ok();
        metadata_obj.set("apiVersion", 1).ok();
        metadata_obj.set("platform", std::env::consts::OS).ok();
        api_obj.set("metadata", metadata_obj).ok();
        Ok(())
    }

    fn setup_ui_api(
        ctx: &rquickjs::Ctx<'_>,
        api_obj: &rquickjs::Object<'_>,
        manifest: &PluginManifest,
    ) -> Result<()> {
        let ui_api = rquickjs::Object::new(ctx.clone()).unwrap();

        if manifest.permissions.contains(&"ui:register-command".to_string()) {
            ui_api
                .set(
                    "registerCommand",
                    rquickjs::Function::new(ctx.clone(), |_opts: rquickjs::Object<'_>| {
                        Ok(())
                    }),
                )
                .ok();
        }

        if manifest.permissions.contains(&"ui:register-view".to_string()) {
            ui_api
                .set(
                    "registerView",
                    rquickjs::Function::new(ctx.clone(), |_opts: rquickjs::Object<'_>| {
                        Ok(())
                    }),
                )
                .ok();
        }

        ui_api
            .set(
                "navigateToNode",
                rquickjs::Function::new(ctx.clone(), move |_id: String| {}),
            )
            .ok();

        ui_api
            .set(
                "showNotification",
                rquickjs::Function::new(ctx.clone(), |msg: String, _opts: rquickjs::Value<'_>| {
                    log::info!("Plugin notification: {}", msg);
                }),
            )
            .ok();

        api_obj.set("ui", ui_api).ok();
        Ok(())
    }

    fn setup_nodes_api(
        ctx: &rquickjs::Ctx<'_>,
        api_obj: &rquickjs::Object<'_>,
        manifest: &PluginManifest,
    ) -> Result<()> {
        let nodes_api = rquickjs::Object::new(ctx.clone()).unwrap();

        if manifest.permissions.contains(&"node:read".to_string()) {
            nodes_api
                .set(
                    "get",
                    rquickjs::Function::new(ctx.clone(), |_id: String| -> Option<rquickjs::Value<'_>> {
                        None
                    }),
                )
                .ok();
            nodes_api
                .set(
                    "getChildren",
                    rquickjs::Function::new(ctx.clone(), |_id: String| -> Vec<rquickjs::Value<'_>> {
                        vec![]
                    }),
                )
                .ok();
        }

        if manifest.permissions.contains(&"node:write".to_string()) {
            nodes_api
                .set(
                    "create",
                    rquickjs::Function::new(ctx.clone(), |_input: rquickjs::Object<'_>| -> Option<rquickjs::Value<'_>> {
                        None
                    }),
                )
                .ok();
            nodes_api
                .set(
                    "update",
                    rquickjs::Function::new(ctx.clone(), |_id: String, _changes: rquickjs::Object<'_>| -> Option<rquickjs::Value<'_>> {
                        None
                    }),
                )
                .ok();
        }

        api_obj.set("nodes", nodes_api).ok();
        Ok(())
    }

    fn setup_storage_api(
        ctx: &rquickjs::Ctx<'_>,
        api_obj: &rquickjs::Object<'_>,
    ) -> Result<()> {
        let storage_api = rquickjs::Object::new(ctx.clone()).unwrap();

        let store: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        let cell = std::cell::RefCell::new(store);

        storage_api
            .set(
                "get",
                rquickjs::Function::new(ctx.clone(), move |key: String| -> Option<String> {
                    cell.borrow().get(&key).cloned()
                }),
            )
            .ok();

        storage_api
            .set(
                "set",
                rquickjs::Function::new(ctx.clone(), move |key: String, value: String| {
                    cell.borrow_mut().insert(key, value);
                }),
            )
            .ok();

        api_obj.set("storage", storage_api).ok();
        Ok(())
    }

    fn setup_events_api(
        ctx: &rquickjs::Ctx<'_>,
        api_obj: &rquickjs::Object<'_>,
    ) -> Result<()> {
        let events_api = rquickjs::Object::new(ctx.clone()).unwrap();

        let counter = std::cell::Cell::new(0u32);

        events_api
            .set(
                "on",
                rquickjs::Function::new(ctx.clone(), move |_event: String, _handler: rquickjs::Function<'_>| -> String {
                    let id = counter.get();
                    counter.set(id + 1);
                    format!("listener-{}", id)
                }),
            )
            .ok();

        events_api
            .set(
                "off",
                rquickjs::Function::new(ctx.clone(), move |_id: String| {}),
            )
            .ok();

        api_obj.set("events", events_api).ok();
        Ok(())
    }

    impl Default for PluginManager {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(not(feature = "plugin-runtime"))]
mod runtime_stub {
    use super::*;

    pub struct PluginManager;

    impl PluginManager {
        pub fn new() -> Self {
            log::warn!("Plugin runtime not compiled (enable 'plugin-runtime' feature)");
            Self
        }

        pub fn load_plugin(&self, _manifest: &PluginManifest, _source: &str) -> Result<()> {
            Err(Error::InvalidOperation(
                "Plugin runtime not available. Rebuild with 'plugin-runtime' feature.".into(),
            ))
        }

        pub fn unload_plugin(&self, _plugin_id: &str) -> Result<()> {
            Ok(())
        }

        pub fn has_permission(&self, _plugin_id: &str, _permission: &str) -> bool {
            false
        }

        pub fn plugin_count(&self) -> usize {
            0
        }
    }

    impl Default for PluginManager {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "plugin-runtime")]
pub use runtime_impl::PluginManager;

#[cfg(not(feature = "plugin-runtime"))]
pub use runtime_stub::PluginManager;

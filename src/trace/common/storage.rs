use super::{Interaction, SharedInteraction};
use std::sync::Arc;

#[cfg(feature = "sync")]
use std::sync::RwLock;

#[cfg(feature = "async")]
use tokio::sync::RwLock;

/// Global storage for the current interaction
#[cfg(feature = "sync")]
static CURRENT_INTERACTION: RwLock<Option<Arc<RwLock<Interaction>>>> = RwLock::new(None);

#[cfg(feature = "async")]
static CURRENT_INTERACTION: RwLock<Option<Arc<RwLock<Interaction>>>> = RwLock::const_new(None);

/// Storage operations for sync mode
#[cfg(feature = "sync")]
pub(in crate::trace) mod sync_ops {
    use super::*;

    /// Gets the last interaction if any
    pub fn get_last_interaction() -> Option<SharedInteraction> {
        let guard = CURRENT_INTERACTION.read().ok()?;
        guard.as_ref().and_then(|arc_rw| {
            let interaction = arc_rw.read().ok()?;
            Some(Arc::new(interaction.clone()))
        })
    }

    /// Starts a new interaction with the given request
    pub fn start_new_interaction(request: String) {
        let new_interaction = Arc::new(RwLock::new(Interaction::new(request)));
        if let Ok(mut guard) = CURRENT_INTERACTION.write() {
            *guard = Some(new_interaction);
        }
    }

    /// Adds a response to the current interaction
    pub fn add_response_to_current(response: String) {
        if let Ok(guard) = CURRENT_INTERACTION.read() {
            if let Some(interaction_arc) = guard.as_ref() {
                if let Ok(mut interaction) = interaction_arc.write() {
                    interaction.add_response(response);
                }
            }
        }
    }

    /// Clears the current interaction (for testing)
    #[cfg(test)]
    pub fn clear() {
        if let Ok(mut guard) = CURRENT_INTERACTION.write() {
            *guard = None;
        }
    }
}

/// Storage operations for async mode
#[cfg(feature = "async")]
pub(in crate::trace) mod async_ops {
    use super::*;

    /// Gets the last interaction if any
    pub async fn get_last_interaction() -> Option<SharedInteraction> {
        let guard = CURRENT_INTERACTION.read().await;
        if let Some(arc_rw) = guard.as_ref() {
            let interaction = arc_rw.read().await;
            Some(Arc::new(interaction.clone()))
        } else {
            None
        }
    }

    /// Starts a new interaction with the given request
    pub async fn start_new_interaction(request: String) {
        let new_interaction = Arc::new(RwLock::new(Interaction::new(request)));
        let mut guard = CURRENT_INTERACTION.write().await;
        *guard = Some(new_interaction);
    }

    /// Adds a response to the current interaction
    pub async fn add_response_to_current(response: String) {
        let guard = CURRENT_INTERACTION.read().await;
        if let Some(interaction_arc) = guard.as_ref() {
            let mut interaction = interaction_arc.write().await;
            interaction.add_response(response);
        }
    }

    /// Clears the current interaction (for testing)
    #[cfg(test)]
    pub async fn clear() {
        let mut guard = CURRENT_INTERACTION.write().await;
        *guard = None;
    }
}

//! Game UI message enums.
//!
//! This module is reserved for future message enums that need to go through
//! the global MessageQueue. Currently, view-specific actions are handled
//! directly within views via context refs, so no message enums are needed.
//!
//! When a view needs to communicate with systems outside the UI layer
//! (other than through mutable context refs), add a message enum here.

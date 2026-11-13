//! MWP core: everything the `mwp` CLI does, without printing and without git.
//! State is plain files, so several agents (and a GUI) can work on one project at once.

pub mod doc;
pub mod lint;
pub mod ops;
pub mod scaffold;
pub mod state;

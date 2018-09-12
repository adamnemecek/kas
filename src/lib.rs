//! Mygui lib

// TODO: for now there are many unused things
#![allow(unused)]

#[doc(hidden)]
pub extern crate cassowary as cw;    // used by macros

extern crate glib;
extern crate gdk;
extern crate gtk;
extern crate gtk_sys;

pub mod event;
pub mod widget;
pub mod toolkit;

mod util;

pub use util::{Coord, Rect};

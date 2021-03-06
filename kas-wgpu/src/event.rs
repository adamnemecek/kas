// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Event handling

use log::{debug, error, trace};
use std::time::Instant;

use winit::event::{Event, StartCause};
use winit::event_loop::{ControlFlow, EventLoopWindowTarget};

use kas::{theme, TkAction};

use crate::draw::DrawPipe;
use crate::{SharedState, Window};

pub(crate) struct Loop<T: theme::Theme<DrawPipe>> {
    windows: Vec<Window<T::Window>>,
    shared: SharedState<T>,
    resumes: Vec<(Instant, usize)>,
}

impl<T: theme::Theme<DrawPipe>> Loop<T> {
    pub(crate) fn new(windows: Vec<Window<T::Window>>, shared: SharedState<T>) -> Self {
        Loop {
            windows,
            shared,
            resumes: vec![],
        }
    }

    pub(crate) fn handle<U>(
        &mut self,
        event: Event<U>,
        elwt: &EventLoopWindowTarget<U>,
        control_flow: &mut ControlFlow,
    ) {
        use Event::*;
        let (i, (action, new_windows)) = match event {
            WindowEvent { window_id, event } => 'outer: loop {
                for (i, window) in self.windows.iter_mut().enumerate() {
                    if window.window.id() == window_id {
                        break 'outer (i, window.handle_event(&mut self.shared, event));
                    }
                }
                return;
            },

            DeviceEvent { .. } => return, // windows handle local input; we do not handle global input
            UserEvent(_) => return,       // we have no handler for user events

            NewEvents(cause) => {
                match cause {
                    StartCause::ResumeTimeReached {
                        requested_resume, ..
                    } => {
                        debug!("Wakeup: timer (requested: {:?})", requested_resume);

                        let item = self
                            .resumes
                            .first()
                            .cloned()
                            .unwrap_or_else(|| panic!("timer wakeup without resume"));
                        assert_eq!(item.0, requested_resume);

                        let (action, resume) = self.windows[item.1].timer_resume(requested_resume);
                        if let Some(instant) = resume {
                            self.resumes[0].0 = instant;
                            self.resumes.sort_by_key(|item| item.0);
                            trace!("Requesting resume at {:?}", self.resumes[0].0);
                            *control_flow = ControlFlow::WaitUntil(self.resumes[0].0);
                        } else {
                            self.resumes.remove(0);
                        }

                        (item.1, (action, vec![]))
                    }

                    StartCause::Init => {
                        debug!("Wakeup: init");

                        for (i, window) in self.windows.iter_mut().enumerate() {
                            if let Some(instant) = window.init() {
                                self.resumes.push((instant, i));
                            }
                        }
                        self.resumes.sort_by_key(|item| item.0);
                        if let Some(first) = self.resumes.first() {
                            trace!("Requesting resume at {:?}", first.0);
                            *control_flow = ControlFlow::WaitUntil(first.0);
                        } else {
                            *control_flow = ControlFlow::Wait;
                        }
                        return;
                    }
                    _ => return, // we can ignore these events
                }
            }

            EventsCleared | LoopDestroyed | Suspended | Resumed => return,
        };

        // Create and init() any new windows.
        let mut have_new_resumes = false;
        for widget in new_windows {
            debug!("Adding window {}", widget.title());
            match winit::window::Window::new(elwt) {
                Ok(window) => {
                    window.set_title(widget.title());
                    let mut win = Window::new(&mut self.shared, window, widget);
                    if let Some(instant) = win.init() {
                        self.resumes.push((instant, self.windows.len()));
                        have_new_resumes = true;
                    }
                    self.windows.push(win);
                }
                Err(e) => {
                    error!("Unable to create window: {}", e);
                }
            };
        }
        if have_new_resumes {
            self.resumes.sort_by_key(|item| item.0);
            if let Some(first) = self.resumes.first() {
                trace!("Requesting resume at {:?}", first.0);
                *control_flow = ControlFlow::WaitUntil(first.0);
            } else {
                *control_flow = ControlFlow::Wait;
            }
        }

        match action {
            TkAction::None => (),
            TkAction::Redraw => self.windows[i].window.request_redraw(),
            TkAction::Reconfigure => self.windows[i].reconfigure(),
            TkAction::Close => {
                self.windows.remove(i);
                if self.windows.is_empty() {
                    *control_flow = ControlFlow::Exit;
                } else {
                    // update window indices in self.resumes!
                    for resume in &mut self.resumes {
                        if resume.1 >= i {
                            resume.1 -= 1;
                        }
                    }
                }
            }
            TkAction::CloseAll => *control_flow = ControlFlow::Exit,
        }
    }
}

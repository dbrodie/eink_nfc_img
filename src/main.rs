//! DMPL0154FN1 4-Color E-Ink NFC Writer for Flipper Zero
//!
//! Writes images to GoodDisplay DMPL0154FN1 NFC-powered e-ink tags.

#![no_std]
#![no_main]

extern crate alloc;
extern crate flipperzero_alloc;

mod image;
mod protocol;

use alloc::boxed::Box;
use core::ffi::c_void;
use core::ptr::null_mut;

use flipperzero_rt::{entry, manifest};
use flipperzero_sys as sys;

// App manifest
manifest!(
    name = "DMPL0154FN1",
    app_version = 1,
    has_icon = false,
);
entry!(main);

/// Application state
struct App {
    view_dispatcher: *mut sys::ViewDispatcher,
    submenu: *mut sys::Submenu,
    widget: *mut sys::Widget,
    gui: *mut sys::Gui,
    image_data: Option<Box<[u8; protocol::IMAGE_DATA_SIZE]>>,
    current_view: u32,
}

/// View IDs
const VIEW_MENU: u32 = 0;
const VIEW_WIDGET: u32 = 1;

/// Menu item IDs
const MENU_SELECT_IMAGE: u32 = 0;
const MENU_WRITE_TAG: u32 = 1;
const MENU_ABOUT: u32 = 2;

/// Helper macro for C string literals (returns *const c_char)
macro_rules! c_str {
    ($s:expr) => {
        concat!($s, "\0").as_ptr() as *const core::ffi::c_char
    };
}

impl App {
    fn new() -> Self {
        Self {
            view_dispatcher: null_mut(),
            submenu: null_mut(),
            widget: null_mut(),
            gui: null_mut(),
            image_data: None,
            current_view: VIEW_MENU,
        }
    }

    unsafe fn init(&mut self) -> bool {
        unsafe {
            // Allocate view dispatcher
            self.view_dispatcher = sys::view_dispatcher_alloc();
            if self.view_dispatcher.is_null() {
                return false;
            }

            // Allocate submenu
            self.submenu = sys::submenu_alloc();
            if self.submenu.is_null() {
                return false;
            }

            // Allocate widget for status display
            self.widget = sys::widget_alloc();
            if self.widget.is_null() {
                return false;
            }

            // Add menu items
            sys::submenu_add_item(
                self.submenu,
                c_str!("Select Image"),
                MENU_SELECT_IMAGE,
                Some(menu_callback),
                self as *mut _ as *mut c_void,
            );
            sys::submenu_add_item(
                self.submenu,
                c_str!("Write to Tag"),
                MENU_WRITE_TAG,
                Some(menu_callback),
                self as *mut _ as *mut c_void,
            );
            sys::submenu_add_item(
                self.submenu,
                c_str!("About"),
                MENU_ABOUT,
                Some(menu_callback),
                self as *mut _ as *mut c_void,
            );

            // Add views to dispatcher
            sys::view_dispatcher_add_view(
                self.view_dispatcher,
                VIEW_MENU,
                sys::submenu_get_view(self.submenu),
            );
            sys::view_dispatcher_add_view(
                self.view_dispatcher,
                VIEW_WIDGET,
                sys::widget_get_view(self.widget),
            );

            // Get GUI record
            self.gui = sys::furi_record_open(c_str!("gui")) as *mut sys::Gui;

            // Attach to GUI
            sys::view_dispatcher_attach_to_gui(
                self.view_dispatcher,
                self.gui,
                sys::ViewDispatcherTypeFullscreen,
            );

            // Enable queue for custom events
            sys::view_dispatcher_enable_queue(self.view_dispatcher);

            // Set navigation callback
            sys::view_dispatcher_set_navigation_event_callback(
                self.view_dispatcher,
                Some(navigation_callback),
            );
            sys::view_dispatcher_set_event_callback_context(
                self.view_dispatcher,
                self as *mut _ as *mut c_void,
            );

            true
        }
    }

    unsafe fn run(&mut self) {
        unsafe {
            // Show menu
            sys::view_dispatcher_switch_to_view(self.view_dispatcher, VIEW_MENU);

            // Run event loop
            sys::view_dispatcher_run(self.view_dispatcher);
        }
    }

    unsafe fn cleanup(&mut self) {
        unsafe {
            // Remove views
            sys::view_dispatcher_remove_view(self.view_dispatcher, VIEW_MENU);
            sys::view_dispatcher_remove_view(self.view_dispatcher, VIEW_WIDGET);

            // Free resources
            if !self.submenu.is_null() {
                sys::submenu_free(self.submenu);
            }
            if !self.widget.is_null() {
                sys::widget_free(self.widget);
            }
            if !self.view_dispatcher.is_null() {
                sys::view_dispatcher_free(self.view_dispatcher);
            }

            // Close GUI record
            sys::furi_record_close(c_str!("gui"));
        }
    }

    unsafe fn show_message(&mut self, title: *const core::ffi::c_char, message: *const core::ffi::c_char) {
        unsafe {
            sys::widget_reset(self.widget);
            sys::widget_add_string_element(
                self.widget,
                64,
                10,
                sys::AlignCenter,
                sys::AlignTop,
                sys::FontPrimary,
                title,
            );
            sys::widget_add_string_element(
                self.widget,
                64,
                32,
                sys::AlignCenter,
                sys::AlignCenter,
                sys::FontSecondary,
                message,
            );
            self.current_view = VIEW_WIDGET;
            sys::view_dispatcher_switch_to_view(self.view_dispatcher, VIEW_WIDGET);
        }
    }

    unsafe fn on_menu_select(&mut self, index: u32) {
        unsafe {
            match index {
                MENU_SELECT_IMAGE => {
                    self.select_image();
                }
                MENU_WRITE_TAG => {
                    self.write_to_tag();
                }
                MENU_ABOUT => {
                    self.show_message(
                        c_str!("DMPL0154FN1 Writer"),
                        c_str!("4-color e-ink NFC tag"),
                    );
                }
                _ => {}
            }
        }
    }

    unsafe fn select_image(&mut self) {
        unsafe {
            // Open dialogs app
            let dialogs = sys::furi_record_open(c_str!("dialogs")) as *mut sys::DialogsApp;

            // Allocate path string
            let path = sys::furi_string_alloc();
            sys::furi_string_set_str(path, c_str!("/ext/eink"));

            // Configure file browser
            let mut options: sys::DialogsFileBrowserOptions = core::mem::zeroed();
            sys::dialog_file_browser_set_basic_options(
                &mut options,
                c_str!(".4ei"),
                null_mut(),
            );
            options.base_path = c_str!("/ext/eink");
            options.hide_dot_files = true;

            // Show file browser
            if sys::dialog_file_browser_show(dialogs, path, path, &options) {
                // Get selected path
                let path_ptr = sys::furi_string_get_cstr(path);

                // Try to load the image
                match image::load_4ei_file(path_ptr) {
                    Ok(data) => {
                        self.image_data = Some(data);
                        self.show_message(c_str!("Image Loaded"), c_str!("Ready to write"));
                    }
                    Err(_) => {
                        self.show_message(c_str!("Error"), c_str!("Failed to load image"));
                    }
                }
            }

            // Cleanup
            sys::furi_string_free(path);
            sys::furi_record_close(c_str!("dialogs"));
        }
    }

    unsafe fn write_to_tag(&mut self) {
        unsafe {
            if self.image_data.is_none() {
                self.show_message(c_str!("No Image"), c_str!("Select an image first"));
                return;
            }

            self.show_message(c_str!("Writing..."), c_str!("Hold tag near Flipper"));

            // Get image data
            let data = self.image_data.as_ref().unwrap();

            // Create protocol handler
            let mut proto = protocol::Dmpl0154Protocol::new();

            // Attempt to write
            match proto.write_image(data.as_ref()) {
                Ok(()) => {
                    self.show_message(c_str!("Success!"), c_str!("Image written to tag"));
                }
                Err(e) => {
                    let msg = match e {
                        protocol::NfcError::DetectFailed => c_str!("Detection failed"),
                        protocol::NfcError::TransmitFailed => c_str!("Transmit failed"),
                        protocol::NfcError::AllocFailed => c_str!("Alloc failed"),
                    };
                    self.show_message(c_str!("Error"), msg);
                }
            }

            proto.cleanup();
        }
    }
}

/// Menu item callback
unsafe extern "C" fn menu_callback(context: *mut c_void, index: u32) {
    unsafe {
        let app = &mut *(context as *mut App);
        app.on_menu_select(index);
    }
}

/// Navigation callback (back button)
unsafe extern "C" fn navigation_callback(context: *mut c_void) -> bool {
    unsafe {
        let app = &mut *(context as *mut App);

        if app.current_view == VIEW_MENU {
            // On main menu, exit the app
            sys::view_dispatcher_stop(app.view_dispatcher);
        } else {
            // Go back to menu
            app.current_view = VIEW_MENU;
            sys::view_dispatcher_switch_to_view(app.view_dispatcher, VIEW_MENU);
        }

        // Return true to indicate event was handled
        true
    }
}

/// Main entry point (v0.16.0 signature)
fn main(_args: Option<&core::ffi::CStr>) -> i32 {
    let mut app = App::new();

    unsafe {
        if !app.init() {
            return -1;
        }

        app.run();
        app.cleanup();
    }

    0
}

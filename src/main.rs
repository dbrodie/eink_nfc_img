//! E-Ink NFC Writer for Flipper Zero
//!
//! Writes images to GoodDisplay NFC-powered e-ink tags.
//! Supports multiple display types and protocols.

#![no_std]
#![no_main]

extern crate alloc;
extern crate flipperzero_alloc;

mod image;
mod protocol_bwry;
mod protocol_common;
mod protocol_genb;
mod tag_type;

use core::ffi::c_void;
use core::ptr::null_mut;

use flipperzero_rt::{entry, manifest};
use flipperzero_sys as sys;

use image::AnyImage;
use tag_type::{Protocol, TagType};

// App manifest
manifest!(
    name = "E-Ink NFC",
    app_version = 1,
    has_icon = false,
);
entry!(main);

/// Application state
struct App {
    view_dispatcher: *mut sys::ViewDispatcher,
    submenu: *mut sys::Submenu,
    tag_submenu: *mut sys::Submenu,
    write_submenu: *mut sys::Submenu,
    widget: *mut sys::Widget,
    gui: *mut sys::Gui,
    selected_tag: Option<&'static TagType>,
    image_data: Option<AnyImage>,
    current_view: u32,
}

/// View IDs
const VIEW_MENU: u32 = 0;
const VIEW_TAG_MENU: u32 = 1;
const VIEW_WRITE_MENU: u32 = 2;
const VIEW_WIDGET: u32 = 3;

/// Main menu item IDs
const MENU_SELECT_IMAGE: u32 = 0;
const MENU_ABOUT: u32 = 1;

/// Write menu item IDs
const WRITE_MENU_WRITE: u32 = 0;
const WRITE_MENU_CANCEL: u32 = 1;

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
            tag_submenu: null_mut(),
            write_submenu: null_mut(),
            widget: null_mut(),
            gui: null_mut(),
            selected_tag: None,
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

            // Allocate main submenu
            self.submenu = sys::submenu_alloc();
            if self.submenu.is_null() {
                return false;
            }

            // Allocate tag selection submenu
            self.tag_submenu = sys::submenu_alloc();
            if self.tag_submenu.is_null() {
                return false;
            }

            // Allocate write submenu
            self.write_submenu = sys::submenu_alloc();
            if self.write_submenu.is_null() {
                return false;
            }

            // Allocate widget for status display
            self.widget = sys::widget_alloc();
            if self.widget.is_null() {
                return false;
            }

            // Add main menu items
            sys::submenu_add_item(
                self.submenu,
                c_str!("Select Image"),
                MENU_SELECT_IMAGE,
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

            // Add tag selection menu items
            for (idx, tag) in TagType::ALL.iter().enumerate() {
                // Create C string for tag name
                // Note: tag.name is &'static str, we need to make it a C string
                let name_cstr = match idx {
                    0 => c_str!("1.54inch e-Paper Y"),
                    1 => c_str!("1.54inch e-Paper B"),
                    _ => c_str!("Unknown"),
                };
                sys::submenu_add_item(
                    self.tag_submenu,
                    name_cstr,
                    idx as u32,
                    Some(tag_menu_callback),
                    self as *mut _ as *mut c_void,
                );
            }

            // Add write menu items
            sys::submenu_add_item(
                self.write_submenu,
                c_str!("Write to Tag"),
                WRITE_MENU_WRITE,
                Some(write_menu_callback),
                self as *mut _ as *mut c_void,
            );
            sys::submenu_add_item(
                self.write_submenu,
                c_str!("Cancel"),
                WRITE_MENU_CANCEL,
                Some(write_menu_callback),
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
                VIEW_TAG_MENU,
                sys::submenu_get_view(self.tag_submenu),
            );
            sys::view_dispatcher_add_view(
                self.view_dispatcher,
                VIEW_WRITE_MENU,
                sys::submenu_get_view(self.write_submenu),
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
            sys::view_dispatcher_remove_view(self.view_dispatcher, VIEW_TAG_MENU);
            sys::view_dispatcher_remove_view(self.view_dispatcher, VIEW_WRITE_MENU);
            sys::view_dispatcher_remove_view(self.view_dispatcher, VIEW_WIDGET);

            // Free resources
            if !self.submenu.is_null() {
                sys::submenu_free(self.submenu);
            }
            if !self.tag_submenu.is_null() {
                sys::submenu_free(self.tag_submenu);
            }
            if !self.write_submenu.is_null() {
                sys::submenu_free(self.write_submenu);
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

    unsafe fn show_tag_menu(&mut self) {
        unsafe {
            self.current_view = VIEW_TAG_MENU;
            sys::view_dispatcher_switch_to_view(self.view_dispatcher, VIEW_TAG_MENU);
        }
    }

    unsafe fn show_write_menu(&mut self) {
        unsafe {
            self.current_view = VIEW_WRITE_MENU;
            sys::view_dispatcher_switch_to_view(self.view_dispatcher, VIEW_WRITE_MENU);
        }
    }

    unsafe fn show_main_menu(&mut self) {
        unsafe {
            self.current_view = VIEW_MENU;
            sys::view_dispatcher_switch_to_view(self.view_dispatcher, VIEW_MENU);
        }
    }

    unsafe fn on_menu_select(&mut self, index: u32) {
        unsafe {
            match index {
                MENU_SELECT_IMAGE => {
                    // Show tag selection menu first
                    self.show_tag_menu();
                }
                MENU_ABOUT => {
                    self.show_message(
                        c_str!("E-Ink NFC Writer"),
                        c_str!("BWR/BWRY e-ink tags"),
                    );
                }
                _ => {}
            }
        }
    }

    unsafe fn on_tag_menu_select(&mut self, index: u32) {
        unsafe {
            if let Some(tag) = TagType::get(index as usize) {
                self.selected_tag = Some(tag);
                self.select_image();
            }
        }
    }

    unsafe fn on_write_menu_select(&mut self, index: u32) {
        unsafe {
            match index {
                WRITE_MENU_WRITE => {
                    self.write_to_tag();
                }
                WRITE_MENU_CANCEL => {
                    self.image_data = None;
                    self.selected_tag = None;
                    self.show_main_menu();
                }
                _ => {}
            }
        }
    }

    unsafe fn select_image(&mut self) {
        unsafe {
            let tag = match self.selected_tag {
                Some(t) => t,
                None => {
                    self.show_main_menu();
                    return;
                }
            };

            // Open dialogs app
            let dialogs = sys::furi_record_open(c_str!("dialogs")) as *mut sys::DialogsApp;

            // Allocate path string
            let path = sys::furi_string_alloc();
            sys::furi_string_set_str(path, c_str!("/ext"));

            // Configure file browser
            let mut options: sys::DialogsFileBrowserOptions = core::mem::zeroed();
            sys::dialog_file_browser_set_basic_options(
                &mut options,
                c_str!(".bmp"),
                null_mut(),
            );
            options.base_path = c_str!("/ext");
            options.hide_dot_files = true;

            // Show file browser
            if sys::dialog_file_browser_show(dialogs, path, path, &options) {
                // Get selected path
                let path_ptr = sys::furi_string_get_cstr(path);

                // Try to load the image with the appropriate format
                match image::load_bmp(path_ptr, tag.image_format) {
                    Ok(data) => {
                        self.image_data = Some(data);
                        // Cleanup and show write menu
                        sys::furi_string_free(path);
                        sys::furi_record_close(c_str!("dialogs"));
                        self.show_write_menu();
                        return;
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
            let tag = match self.selected_tag {
                Some(t) => t,
                None => {
                    self.show_message(c_str!("Error"), c_str!("No tag type selected"));
                    return;
                }
            };

            if self.image_data.is_none() {
                self.show_message(c_str!("No Image"), c_str!("Select an image first"));
                return;
            }

            // Show writing status
            let status_msg = match tag.protocol {
                Protocol::IsodepBwry => c_str!("Writing BWRY..."),
                Protocol::IsodepGenb => c_str!("Writing BWR..."),
            };
            self.show_message(c_str!("Writing..."), status_msg);

            // Get image data and dispatch to appropriate protocol
            let img = self.image_data.as_ref().unwrap();

            let result = match (tag.protocol, img) {
                (Protocol::IsodepBwry, AnyImage::Bwry(image)) => {
                    let mut proto = protocol_bwry::BwryProtocol::new();
                    proto.write_image(image.as_slice())
                }
                (Protocol::IsodepGenb, AnyImage::Bwr(image)) => {
                    let mut proto = protocol_genb::GenbProtocol::new();
                    proto.write_image(image.as_slice())
                }
                _ => {
                    // This should never happen due to type safety
                    self.show_message(c_str!("Error"), c_str!("Format mismatch"));
                    return;
                }
            };

            match result {
                Ok(()) => {
                    self.show_message(c_str!("Success!"), c_str!("Image written to tag"));
                }
                Err(e) => {
                    let msg = match e {
                        protocol_common::NfcError::DetectFailed => c_str!("Detection failed"),
                        protocol_common::NfcError::TransmitFailed => c_str!("Transmit failed"),
                        protocol_common::NfcError::AllocFailed => c_str!("Alloc failed"),
                    };
                    self.show_message(c_str!("Error"), msg);
                }
            }
        }
    }
}

/// Main menu item callback
unsafe extern "C" fn menu_callback(context: *mut c_void, index: u32) {
    unsafe {
        let app = &mut *(context as *mut App);
        app.on_menu_select(index);
    }
}

/// Tag menu item callback
unsafe extern "C" fn tag_menu_callback(context: *mut c_void, index: u32) {
    unsafe {
        let app = &mut *(context as *mut App);
        app.on_tag_menu_select(index);
    }
}

/// Write menu item callback
unsafe extern "C" fn write_menu_callback(context: *mut c_void, index: u32) {
    unsafe {
        let app = &mut *(context as *mut App);
        app.on_write_menu_select(index);
    }
}

/// Navigation callback (back button)
unsafe extern "C" fn navigation_callback(context: *mut c_void) -> bool {
    unsafe {
        let app = &mut *(context as *mut App);

        match app.current_view {
            VIEW_MENU => {
                // On main menu, exit the app
                sys::view_dispatcher_stop(app.view_dispatcher);
            }
            VIEW_TAG_MENU => {
                // On tag menu, go back to main menu
                app.show_main_menu();
            }
            VIEW_WRITE_MENU => {
                // On write menu, go back to tag menu and clear image
                app.image_data = None;
                app.show_tag_menu();
            }
            _ => {
                // On other views (widget), go back to main menu
                app.image_data = None;
                app.selected_tag = None;
                app.show_main_menu();
            }
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

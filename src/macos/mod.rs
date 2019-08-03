mod rustnsobject;

extern crate objc;
extern crate objc_foundation;
extern crate cocoa;

extern crate fruitbasket;
use self::fruitbasket::FruitApp;

use objc::runtime::Class;
use objc::*;

use self::cocoa::base::{nil, YES};
use self::cocoa::appkit::NSStatusBar;
use self::cocoa::foundation::NSString;
use self::cocoa::appkit::{NSMenu,
                          NSMenuItem,
                          NSImage,
                          NSVariableStatusItemLength,
                          NSStatusItem,
                          NSButton};

use self::rustnsobject::{NSObj, NSObjTrait, NSObjCallbackTrait};

use std::sync::mpsc::Sender;
use std::ptr;
use std::ffi::CStr;
use crate::NSCallback;

pub type Object = objc::runtime::Object;

pub struct OSXStatusBar {
    object: NSObj,
    app: FruitApp,
    status_bar_item: *mut objc::runtime::Object,
    menu_bar: *mut objc::runtime::Object,
    run_count: u32,
}

impl OSXStatusBar {
    pub fn new(tx: Sender<String>) -> OSXStatusBar {
        let mut bar;
        unsafe {
            let nsapp = FruitApp::new();
            nsapp.set_activation_policy(fruitbasket::ActivationPolicy::Prohibited);
            let status_bar = NSStatusBar::systemStatusBar(nil);

            bar = OSXStatusBar {
                app: nsapp,
                status_bar_item: status_bar.statusItemWithLength_(NSVariableStatusItemLength),
                menu_bar: NSMenu::new(nil),
                object: NSObj::alloc(tx),
                run_count: 0,
            };

            // Default mode for menu bar items: blue highlight when selected
            let _: () = msg_send![bar.status_bar_item, setHighlightMode:YES];

            // Set title.  Only displayed if image fails to load.
            let title = NSString::alloc(nil).init_str("connectr");
            NSButton::setTitle_(bar.status_bar_item, title);
            let _: () = msg_send![title, release];

            // Look for icon in OS X bundle if there is one, otherwise current dir.
            // See docs/icons.md for explanation of icon files.
            // TODO: Use the full list of search paths.
            let icon_name = "connectr_80px_300dpi";
            let img_path = match fruitbasket::FruitApp::bundled_resource_path(icon_name, "png") {
                Some(path) => path,
                None => format!("{}.png", icon_name),
            };

            // Set the status bar image.  Switching on setTemplate switches it to
            // using OS X system-style icons that are masked to all white.  I
            // prefer color, but that should maybe be configurable.
            let img = NSString::alloc(nil).init_str(&img_path);
            let icon = NSImage::alloc(nil).initWithContentsOfFile_(img);
            #[cfg(feature = "mac_white_icon")]
                let _: () = msg_send![icon, setTemplate: YES]; // enable to make icon white
            bar.status_bar_item.button().setImage_(icon);
            let _: () = msg_send![img, release];
            let _: () = msg_send![icon, release];

            // Add the same image again as an alternate image.  I'm not sure how the
            // blending is performed, but it behaves differently and better if an
            // alt image is specified.  Without an alt image, the icon darkens too
            // much in 'dark mode' when selected, and is too light in 'light mode'.
            let img = NSString::alloc(nil).init_str(&img_path);
            let icon = NSImage::alloc(nil).initWithContentsOfFile_(img);
            let _: () = msg_send![bar.status_bar_item.button(), setAlternateImage: icon];
            let _: () = msg_send![img, release];
            let _: () = msg_send![icon, release];

            bar.status_bar_item.setMenu_(bar.menu_bar);
            bar.object.cb_fn = Some(Box::new(
                move |s, sender| {
                    let cb = s.get_value(sender);
                    cb(sender, &s.tx);
                }
            ));
        }
        bar
    }

    // TODO: whole API should accept menu option.  this whole thing should
    // be split out into its own recursive menu-builder trait.  this is
    // horrible.
    fn add_item(&mut self, menu: Option<*mut Object>,item: &str, callback: NSCallback, selected: bool) -> *mut Object {
        unsafe {
            let txt = NSString::alloc(nil).init_str(item);
            let quit_key = NSString::alloc(nil).init_str("");
            let app_menu_item = NSMenuItem::alloc(nil)
                .initWithTitle_action_keyEquivalent_(txt, self.object.selector(), quit_key);
            let _: () = msg_send![txt, release];
            let _: () = msg_send![quit_key, release];
            self.object.add_callback(app_menu_item, callback);
            let objc = self.object.take_objc();
            let _: () = msg_send![app_menu_item, setTarget: objc];
            if selected {
                let _: () = msg_send![app_menu_item, setState: 1];
            }
            let item: *mut Object = app_menu_item;
            match menu {
                Some(menu) => { menu.addItem_(app_menu_item); },
                None => { self.menu_bar.addItem_(app_menu_item); }
            }
            let _: () = msg_send![app_menu_item, release];
            item
        }
    }

    pub fn run(&mut self, block: bool) {
        self.run_count += 1;
        unsafe {
            let title = format!("connectr {:}", self.run_count);
            let title = NSString::alloc(nil).init_str(title.as_str());
            NSButton::setTitle_(self.status_bar_item, title);
            let _: () = msg_send![title, release];
        }

        let period = match block {
            true => fruitbasket::RunPeriod::Forever,
            _ => fruitbasket::RunPeriod::Once,
        };

        let _ = self.app.run(period);
    }
}
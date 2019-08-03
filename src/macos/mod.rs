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

pub type Object = objc::runtime::Object;

pub struct OSXStatusBar {
    object: NSObj,
    app: FruitApp,
    status_bar_item: *mut objc::runtime::Object,
    menu_bar: *mut objc::runtime::Object,
}

pub fn status(tx: Sender<String>) {
    unsafe {
        let nsapp = FruitApp::new();
        nsapp.set_activation_policy(fruitbasket::ActivationPolicy::Prohibited);
        let status_bar = NSStatusBar::systemStatusBar(nil);

        let bar = OSXStatusBar {
            app: nsapp,
            status_bar_item: status_bar.statusItemWithLength_(NSVariableStatusItemLength),
            menu_bar: NSMenu::new(nil),
            object: NSObj::alloc(tx),
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
}
# 11. Multiple windows

Date: 2020-06-14

## Status

Living Draft

## Context

Currently, kyute creates a default (platform) window, and has an application trait to provide the UI to draw into that window.

Q: Should we treat windows differently than other widgets? I.e. should we just make a widget a specific kind of window?

Q: What kind of interface do we want for windows?

Q: Should we do a distinction between multiple root windows and modal dialogs?

Q: Should window creation go inside the widget tree? Or outside?
A: for modal windows / menus, it should certainly go through the widget tree

Q: How do we close windows?



```rust
Button::new("label").on_click(|| {
	PopupMenu::new()
		.push_item(Text::new("Item 1"))
		.push_separator()
		.push_item(Text::new("Copy"), cloned!{events, || events.copy()})
		.push_item(Text::new("Cut"), cloned!{events, || events.cut()})
		.push_item(Text::new("Paste") cloned!{events, || events.paste()})
		.push_item(Text::new("Delete"), cloned!{events, || events.delete()});
})
```
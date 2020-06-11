# 8. Widget visual design

Date: 2020-06-06

## Status

Draft

## Context

It's more fun to work on the library if the GUI that it produces looks pretty. In this document we try to design
widget frames that look good, taking inspiration from the UI of Photoshop CS6.


## Implementation

It's a lot of code to draw a box with two borders and gradient fills:
```

fn make_gradient(ctx: &mut DrawContext, a: LinSrgba, b: LinSrgba) -> GradientStopCollection {
    GradientStopCollection::new(
    ctx,
    ColorInterpolationMode::GammaCorrect,
    ExtendMode::Clamp,
    &[(0.0, Srgba::from_linear(a)), (1.0, Srgba::from_linear(b))],
    )
}

fn make_vertical_gradient_brush(ctx: &mut DrawContext, ystart: f64, yend: f64, a: LinSrgba, b: LinSrgba, opacity: f64) -> LinearGradientBrush {
    let grad = make_gradient(ctx, a, b);
    LinearGradientBrush::new(
        ctx,
        &grad,
        (0.0, ystart).into(),
        (0.0, yend).into(),
        opacity,
    )
}

fn draw_outer_highlight(
    ctx: &mut DrawContext,
    focused: bool,
    bounds: Bounds,
    radius: f64,
    env: &Environment)
{
    let frame_highlight_opacity = env.get(FrameOuterHighlightOpacity);

    if focused {
        let brush = env.get(FrameFocusColor).into_brush(ctx);
        ctx.draw_rounded_rectangle(
            bounds.inflate(0.5, 0.5),
            radius,
            radius,
            &brush,
            1.0,
        );
    } else {
        let brush = make_vertical_gradient_brush(ctx, bounds.size.height, 0.8 * bounds.size.height,
                                                 LinSrgba::new(1.0, 1.0, 1.0, 1.0),
                                                 LinSrgba::new(1.0, 1.0, 1.0, 0.0),
                                                 frame_highlight_opacity);
        ctx.draw_rounded_rectangle(
            bounds.inflate(0.5, 0.5),
            radius,
            radius,
            &brush,
            1.0,
        );
    }
}

pub fn draw_button_frame(ctx: &mut DrawContext, style: &FrameStyle, bounds: Bounds, env: &Environment) {
    let raised: LinSrgba = env.get(FrameBgRaisedColor).into_linear();
    let sunken: LinSrgba = env.get(FrameBgSunkenColor).into_linear();
    let radius = env.get(ButtonBorderRadius);

    // ---- draw background ----
    let mut bg_base = raised;
    if style.hovered {
        bg_base = bg_base.lighten(0.2);
    }
    let bg_low = bg_base.darken(0.05);
    let bg_high = bg_base.lighten(0.05);
    let bg_brush = make_vertical_gradient_brush(ctx, bounds.size.height, 0.0,
                                             bg_low, bg_high,
                                                1.0);
    ctx.fill_rounded_rectangle(
        bounds,
        radius,
        radius,
        &bg_brush,
    );

    // ---- top highlight ----
    let top_highlight_brush = Color::new(1.0, 1.0, 1.0, 0.3).into_brush(ctx);
    ctx.fill_rectangle(
        Bounds::new(
            bounds.origin + Offset::new(1.0, 1.0),
            Size::new(bounds.size.width-1.0, 1.0),
        ),
        &top_highlight_brush,
    );

    // ---- draw border ----
    let border_rect = bounds.inflate(-0.5, -0.5);
    let mut border_base = sunken.darken(0.023);
    //let mut border_low = border_base.darken(0.01);
    //let mut border_high = border_base.lighten(0.01);
    let brush = border_base.into_brush(ctx);

    /*let brush = make_vertical_gradient_brush(ctx, bounds.size.height, 0.0,
                                                border_low, border_high,
                                                1.0);*/
    ctx.draw_rounded_rectangle(
        bounds.inflate(-0.5, -0.5),
        radius,
        radius,
        &brush,
        1.0,
    );

    // ---- outer highlight ----
    draw_outer_highlight(ctx, style.focused, bounds, radius, env);
}
```

Several observations:
- There are conversions between `Srgb[a]` and `LinSrgb[a]` because only linear color types support the `Shade` trait which
has the `lighten` and `darken` methods.
- `bounds.inflate(0.5,0.5)` to draw a proper rectangle (note: this does not consider DPI).
- to fill a rectangle with a gradient, you have to create a gradient first, then create a brush, then finally draw the 
function. Same to fill with a color: you need to create a `SolidColorBrush` first. This is because the API of Direct2D 
is low-level, which is not ideal for quickly iterating over designs (`kyute_shell::drawing` is basically Direct2D).
    - It's probably meant as a backend for a higher-level drawing library.
    - This could be alleviated with _generic trait magic_ ✨ in `kyute_shell::drawing` to be able to directly call drawing 
    functions with colors or a gradient stop list. But it will complicate things a bit, and you can only go so far with
    generics.  
    
In conclusion, the Direct2D API is too low-level to design styles. 
A layer on top of D2D to faciliate drawing would make this clear. In the web world, stuff like this is typically handled
by CSS, which is _declarative_: you describe _what it should look like_, and _how it should be drawn_.
 
But do we want CSS in kyute? It has also a bunch of stuff that are not immediately useful to us, like selectors; and 
it also lacks variables, which are useful when designing styles.
A Rust-based DSL is another option, but Rust is not the most ergonomic language to design DSL with, and it lacks any kind
of hot-reload ability.

At a glance, this "declarative drawing system" would have the following characteristics:
- Input: the bounds of the elements to draw, state modifiers (bunch of flags), and an environment (variables), and a target 
  to draw into.
- Output: drawn visual
- Declarative: to draw a border, you don't have to calculate the bounds of the rectangle to draw, you just say 
    `border at 1px outside`
    - this will work regardless of whether the base element is a rectangle, a rounded rectangle, a path, etc. 
      Just like CSS.
- you can draw multiple borders
- a layer on top of `kyute-shell::drawing`
- hot-reloadable

Note that having a "language" at all is not mandatory: I'd be perfectly fine with something like the "Layer Style" 
dialog of Photoshop: it's intuitive, has immediate feedback, requires no documentation... It's however not possible
to do "procedural" stuff like referencing variables, etc.
- Maybe it's the right job for a projectional editor? (finally getting to use MPS for something)

Overview of such a language:
- Base shape: what to draw (the geometry) inside the given bounds (bounds are always rectangles)
    - rect
    - rounded rect
    - custom shape (not necessarily closed)
    - bitmap mask (with positioning and/or stretch) 
- Expressions: can evaluate to a scalar, a 2D offset, a color, a color with alpha, a gradient
    - Gradient: list of stops, with position and color
- Shape add-ons:
    - Fill: how to fill the shape
    - Shadow: inner and drop shadows
    - Glow: inner and outer glows
    - Conditionals: draw only if condition is true 
    - Bevel and emboss
    - Pattern overlay
    - Gradient expressions
    - Color expressions
        - lighten
        - darken
        - rgb
        - rgba
    - Colorize
    
- Fill: color | gradient

- Shape
    - Add-ons: border
    
- For now, let's just build the "Layer style" model in Rust (with expressions, etc.) and then create a GUI for it
 (bootstrapping ftw)
 
```rust
pub enum Value<T> {
    Constant(T),
    Expr(String),
}

pub enum ElementShape {
    Rect,
    RoundedRect(Value<f64>)
}

pub struct ElementStyle {
    shape: ElementShape,
    decorations: Vec<Decoration>,    
}
pub struct Decoration {
    condition: Value<bool>,
    deco: DecorationKind
}

```

Do we want the struct to contain a style ready to draw (with all colors/sizes resolved to concrete values)? 
Or do we want it to be a template?
What about animations?
- We don't want variables for everything
    - For example, sizes should be fixed
    - Colors as well? => can't design a style and then vary colors; or use the same color for different decorations
        - Maybe a "palette" could be interesting
        - Define a palette beforehand and then use the colors in the palette; no expressions, just a color reference
        - Palette is associated to a "StyleCollection"
- Animations:
    - we certainly want them (for some parameters: not. opacity).

### Loading and using bitmaps
In CSS: 
- `url(...)`
    - also, data URIs: `data:[<mime type>][;charset=<charset>][;base64],<encoded data>`

Follow suit:
- `file://./icons/copy.png`
- `res:icons/copy`
- `icon:copy`


/// Palettes:
/// - a default palette is provided with the style
/// - the palette can be overriden during rendering (e.g. to switch to "dark mode") -> in environment, key ThemePaletteIndex

/// A StyleSet is a stack of styles: to determine the style to render, all
/// styles elements in the stack are merged, the next one overriding the previous one, depending
/// on the state mask; borders accumulate, but other effects override.

/// E.g: style set for button:
/// - Button (default) (????)
/// - Button (!hovered,!active) (00??) - styles
/// - Button (hovered+!active) (10??) - override fill
/// - Button (!hovered+active) (01??) - override fill, override drop shadow
/// - Button (hovered+active) (11??) - override fill
/// - Button (disabled) - override intensity

## Integration with kyute

- should a StyleCollection be passed via the environment?
    - can a StyleCollection be overriden for child widgets?
        - probably not (maybe change the palette)
        - if overriding, can't override only one style, cumbersome
        - pass along the paintctx? 
use iced_core::{
    self, 
    border::Border, 
    layout::{self, Layout, Limits, Node}, 
    mouse::{self, Cursor}, 
    widget::{tree::{self, Tree}, Operation}, 
    Alignment, Background, Clipboard, Color, Element, Length, 
    Padding, Rectangle, Shadow, Shell, Size, Theme, Widget, event, 
    Vector, renderer, overlay,  
};

use crate::styles::{equal_radius, get_rgb_color};

pub struct HovContainer<'a, Message, Theme = iced::Theme, Renderer = iced::Renderer,> 
    where 
        Renderer: iced_core::renderer::Renderer,
        Theme: Catalog,
    {
    content: Vec<Element<'a, Message, Theme, Renderer>>,
    padding: Padding,
    height: Length,
    width: Length,
    on_hover: Option<OnHover<'a, Message>>,
    on_exit: Option<OnExit<'a, Message>>,
    hover_col: Option<iced::Color>,
    theme: Theme::Class<'a>
}

impl<'a, Message, Theme, Renderer> HovContainer<'a, Message, Theme, Renderer>
    where 
        Renderer: iced_core::renderer::Renderer,
        Theme: Catalog,
        {
        pub fn new() -> Self{
            let content = Vec::new();
            Self {
                content,
                padding: DEFAULT_PADDING,
                height: Length::Shrink,
                width: Length::Shrink,
                on_hover: None,
                on_exit: None,
                hover_col: None,  
                theme: Theme::default(), 
            }
        }

        pub fn with_content(content: Vec<Element<'a, Message, Theme, Renderer >>) -> Self{
            Self {
                content,
                padding: DEFAULT_PADDING,
                height: Length::Shrink,
                width: Length::Shrink,
                on_hover: None,
                on_exit: None,
                hover_col: None,  
                theme: Theme::default(), 
            }
        }

        pub fn with_capacity(capacity: usize) -> Self {
            Self::with_content(Vec::with_capacity(capacity))
        }

        pub fn with_children(children: impl IntoIterator<Item = Element<'a, Message, Theme, Renderer>>) -> Self {
            let iterator = children.into_iter();
            
            Self::with_capacity(iterator.size_hint().0).extend(iterator)
        }

        pub fn width(mut self, width: Length) -> Self {
            self.width = width;
            self
        }

        pub fn height(mut self, height: Length) -> Self {
            self.height = height;
            self
        }

        pub fn on_hover(mut self, message: Message) -> Self{
            self.on_hover = Some(OnHover::Direct(message));
            self
        } 

        pub fn on_exit(mut self, message: Message) -> Self {
            self.on_exit = Some(OnExit::Direct(message));
            self
        }

        pub fn padding<P>(mut self, padding: P) -> Self 
            where
                P: Into<Padding> {
                self.padding = padding.into();
                self
        }

        pub fn hover_color(mut self, color: Color) -> Self {
            self.hover_col = Some(color);
            self
        }

        pub fn push(
                mut self,
                child: impl Into<Element<'a, Message, Theme, Renderer>>,
            ) -> Self {
                let child = child.into();
                let child_size = child.as_widget().size_hint();

                self.width = self.width.enclose(child_size.width);
                self.height = self.height.enclose(child_size.height);

                self.content.push(child);
                self
            }
        
        pub fn extend(
            self,
            children: impl IntoIterator<Item = Element<'a, Message, Theme, Renderer>>) -> Self {
                children.into_iter().fold(self, Self::push)
            }

        pub fn style(mut self, style: impl Fn(&Theme, Status) -> Style + 'a) -> Self 
        where
            Theme::Class<'a>: From<StyleFn<'a, Theme>>,
        {
            self.theme = (Box::new(style) as StyleFn<'a, Theme>).into();
            self
        }
        
        

}

enum OnHover <'a, Message> {
    Direct(Message),
    Closure(Box<dyn Fn() -> Message + 'a>),
}

enum OnExit <'a, Message> {
    Direct(Message),
    Closure(Box<dyn Fn() -> Message + 'a>),
}

impl <'a, Message: Clone> OnHover <'a, Message> {
    fn get(&self) -> Message {
        match self {
            OnHover::Direct(message) => message.clone(),
            OnHover::Closure(f) => f(),
        }
    }
}

impl <'a, Message: Clone> OnExit <'a, Message> {
    fn get(&self) -> Message {
        match self {
            OnExit::Direct(message) => message.clone(),
            OnExit::Closure(f) => f(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct State {
    is_hovered: bool,
}

impl <'a, Message, Theme, Renderer> Widget<Message, Theme, Renderer> 
    for HovContainer<'a, Message, Theme, Renderer>
where 
    Message: 'a + Clone,
    Renderer: 'a + iced_core::renderer::Renderer,
    Theme: Catalog,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.content);
    }
    
    fn size(&self) -> Size<Length>{
        Size {
            width: self.width,
            height: self.height,
        }
    }

    fn children(&self) -> Vec<Tree> {
        self.content.iter().map(Tree::new).collect()
    }

    fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        layout::flex::resolve(
            layout::flex::Axis::Horizontal,
            renderer,
            limits,
            self.width,
            self.height,
            self.padding,
            2.0,
            Alignment::Center,
            &self.content,
            &mut tree.children,
        )
    }

    fn operate(&self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        operation.container(None, layout.bounds(), &mut |operation| {
            self.content.iter().zip(&mut tree.children).zip(layout.children())
            .for_each(|((child, state), layout)| {
                child.as_widget().operate(state, layout, renderer, operation);
            })
        })
    }

    fn on_event(&mut self, tree: &mut Tree, event: event::Event, layout: Layout<'_>, cursor: mouse::Cursor, renderer: &Renderer, clipboard: &mut dyn Clipboard, shell: &mut Shell<'_, Message>, viewport: &Rectangle,) -> event::Status {
        
        // self.content.iter_mut()
        self.content
        .iter_mut()
        .zip(&mut tree.children)
        .zip(layout.children())
        .map(|((child, state), layout)| {
            child.as_widget_mut().on_event(
                state,
                event.clone(),
                layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            )
        })
        .fold(event::Status::Ignored, event::Status::merge);

        if let Some(on_exit) = self.on_exit.as_ref().map(OnExit::get) {
            if let Some(on_hover) = self.on_hover.as_ref().map(OnHover::get)
            {
                match tree.state {
                    tree::State::None => {
                        // eprintln!("State Empty");
                        return event::Status::Ignored;
                    }
                    tree::State::Some(_) => {
                        let state = tree.state.downcast_mut::<State>();
                        // eprintln!("Something");
                        let was_hovered = state.is_hovered;
                        let mut now_hovered = false;
                        if let Some(position) = cursor.position() {
                            now_hovered = layout.bounds().contains(position);
                        }

                        match (was_hovered, now_hovered) {
                            (true, false) => {
                                state.is_hovered = now_hovered;
                                shell.publish(on_exit);
                                return event::Status::Captured;
                            }
                            (false, true) => {
                                state.is_hovered = now_hovered;
                                shell.publish(on_hover);
                                return event::Status::Captured;
                            }
                            _ => {
                                return event::Status::Ignored;
                            }
                        }
                    }
                }
            }
            else {
                event::Status::Ignored
            }
        }
        else {
            match tree.state {
                tree::State::None => {
                    // eprintln!("State Empty");
                    return event::Status::Ignored;
                }
                tree::State::Some(_) => {
                    let state = tree.state.downcast_mut::<State>();
                    // eprintln!("Something");
                    let was_hovered = state.is_hovered;
                    let mut now_hovered = false;
                    if let Some(position) = cursor.position() {
                        now_hovered = layout.bounds().contains(position);
                    }

                    match (was_hovered, now_hovered) {
                        (true, false) => {
                            state.is_hovered = now_hovered;
                            return event::Status::Ignored;
                        }
                        (false, true) => {
                            state.is_hovered = now_hovered;
                            return event::Status::Ignored;
                        }
                        _ => {
                            return event::Status::Ignored;
                        }
                    }
                }
            }
            event::Status::Captured
        }

    }
    fn draw(&self, tree: &Tree, renderer: &mut Renderer, theme: &Theme, _style: &renderer::Style, layout: Layout, cursor: Cursor, rect: &Rectangle) {
        let bounds = layout.bounds();
        let content_layout = layout.children().next().unwrap();
        let is_mouse_over = cursor.is_over(bounds);
        
        let status = if self.on_hover.is_none() {
            Status::Disabled
        } else if is_mouse_over && tree.state.downcast_ref::<State>().is_hovered{
            Status::Hovered
        } else {
            Status::NotHovered
        };

        let style = theme.style(&self.theme, status);

        if style.background.is_some() ||
            style.border.width > 0.0 ||
            style.shadow.color.a > 0.0
        {
            renderer.fill_quad(
                renderer::Quad {
                    bounds,
                    border: style.border,
                    shadow: style.shadow,
                },
                style.background.unwrap_or(Background::Color(Color::TRANSPARENT))
            );
        }

        for ((child, state), layout) in self
                .content
                .iter()
                .zip(&tree.children)
                .zip(layout.children())
            {
                child.as_widget().draw(
                    &tree.children[0],
                    renderer,
                    theme,
                    &renderer::Style {
                        text_color: style.text_color,
                    },
                    content_layout,
                    cursor,
                    &rect,
                );

            }
        }

    fn overlay<'b> (
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        translation: Vector
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        overlay::from_children(
            &mut self.content,
            tree, 
            layout, 
            renderer, 
            translation,
        )
    }
}

impl <'a, Message, Theme, Renderer> From <HovContainer<'a, Message, Theme, Renderer>>
    for Element<'a, Message, Theme, Renderer> 
where
    Message: Clone + 'a,
    Theme: Catalog + 'a,
    Renderer: iced_core::Renderer + 'a,
{
    fn from (hov_cont: HovContainer<'a, Message, Theme, Renderer>) -> Self {
        Self::new(hov_cont)
    }
}

impl <'a, Message, Theme, Renderer: iced_core::Renderer> FromIterator <Element<'a, Message, Theme, Renderer>>
    for HovContainer<'a, Message, Theme, Renderer>
    where 
        Theme: Catalog,
{
    fn from_iter<T: IntoIterator<Item = Element<'a, Message, Theme, Renderer>>>(iter: T) -> Self {
        Self::with_children(iter)
    }
}

pub(crate) const DEFAULT_PADDING: Padding = Padding {
    top: 5.0,
    bottom: 5.0,
    right: 10.0,
    left: 10.0,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status{
    Disabled,
    Hovered,
    NotHovered,
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    pub background: Option<Background>,
    pub text_color: Color,
    pub border: Border,
    pub shadow: Shadow,
}

impl Style {
    pub fn with_border_color(self, border: Border) -> Self {
        Self {
            border: border,
            ..self
        }
    }
}

impl Default for Style {
    fn default() -> Self {
        Self {
            background: None,
            text_color: Color::WHITE,
            border: Border::default(),
            shadow: Shadow::default(),
        }
    }
}

pub trait Catalog {
    type Class<'a>;

    fn default<'a>() -> Self::Class<'a>;

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style;
}

pub type StyleFn<'a, Theme> = Box<dyn Fn(&Theme, Status) -> Style + 'a>;

impl Catalog for Theme {
    type Class<'a> = StyleFn<'a, Self>;

    fn default<'a>() -> Self::Class<'a> {
        Box::new(primary)
    }

    fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
        class(self, status)
    }
}

pub fn primary(theme: &Theme, status: Status) -> Style {
    let _palette = theme.extended_palette();
    match status {
        Status::Disabled => {
            Style {
                background: Some(Background::Color(Color::TRANSPARENT)),
                text_color: get_rgb_color(255, 255, 255),
                border: Border {color: get_rgb_color(100, 100, 100), width: 2.0, radius: equal_radius(5)},
                ..Default::default()
            }
        },
        Status::Hovered => {
            Style {
                background: Some(Background::Color(Color::TRANSPARENT)),
                text_color: get_rgb_color(255, 255, 255),
                border: Border {color: get_rgb_color(0, 0, 255), width: 2.0, radius: equal_radius(5)},
                ..Default::default()
            }
        },
        Status::NotHovered => {
            Style {
                background: Some(Background::Color(Color::TRANSPARENT)),
                text_color: get_rgb_color(255, 255, 255),
                border: Border {color: get_rgb_color(150, 150, 150), width: 2.0, radius: equal_radius(5)},
                ..Default::default()
            }
        }
    }
}

pub fn auto_style(unhov_color: Color, hov_color: Color, width: i32, radius: u32) -> impl Fn(&Theme, Status) -> Style {
    move |_theme: &Theme, status | {
        match status {
            Status::Disabled => {
                let border = Border {
                    color: unhov_color,
                    width: width as f32,
                    radius: equal_radius(radius),
                };
                Style::with_border_color(Style{..Default::default()}, border)
            },
            Status::Hovered => {
                let border = Border {
                    color: hov_color,
                    width: width as f32,
                    radius: equal_radius(radius),
                };
                Style::with_border_color(Style{..Default::default()}, border)
            },
            Status::NotHovered => {
                let border = Border {
                    color: unhov_color,
                    width: width as f32,
                    radius: equal_radius(radius),
                };
                Style::with_border_color(Style{..Default::default()}, border)
            },
        }
    }
}


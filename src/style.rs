use iced::{Border, Color, Shadow, Theme, border::Radius};

const SELECTED_COLOR: Color = Color::from_rgb8(169, 207, 128);
const LIGHT_GRAY: Color = Color::from_rgb8(240, 240, 240);
const DESELECTED_COLOR: Color = Color::from_rgb8(222, 222, 222);
const TEXT_COLOR: Color = Color::from_rgb8(0, 0, 0);
const BORDER_COLOR: Color = Color::from_rgb8(125, 125, 125);
// const THUMBNAIL_COLOR: Color = Color::from_rgb8(60,60,60);
const DARK_ORANGE: Color = Color::from_rgb8(168, 105, 50);
const LIGHT_ORANGE: Color = Color::from_rgb8(237, 154, 81);
const STANDARD_BUTTON: Color = Color::from_rgb8(134, 168, 96);
const ADD: Color = Color::from_rgb8(119, 174, 230);
const SUBTRACT: Color = Color::from_rgb8(79, 151, 224);
const PROGRESS_ON: Color = Color::from_rgb8(134, 168, 96);
const PROGRESS_OFF: Color = Color::from_rgb8(100, 100, 100);
const BOTTOM_BAR: Color = Color::from_rgb8(50, 50, 50);
const LIGHT_TEXT: Color = Color::from_rgb8(225, 225, 225);

pub fn selected_button(
    _theme: &Theme,
    _status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: Some(iced::Background::Color(SELECTED_COLOR)),
        text_color: TEXT_COLOR,
        border: Border {
            color: BORDER_COLOR,
            width: 0.0,
            radius: Radius::new(5),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn deselected_button(
    _theme: &Theme,
    _status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: Some(iced::Background::Color(DESELECTED_COLOR)),
        text_color: TEXT_COLOR,
        border: Border {
            color: BORDER_COLOR,
            width: 0.0,
            radius: Radius::new(5),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn standard_button(
    _theme: &Theme,
    _status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: Some(iced::Background::Color(STANDARD_BUTTON)),
        text_color: TEXT_COLOR,
        border: Border {
            color: BORDER_COLOR,
            width: 0.0,
            radius: Radius::new(5),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn add_button(
    _theme: &Theme,
    _status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: Some(iced::Background::Color(ADD)),
        text_color: TEXT_COLOR,
        border: Border {
            color: BORDER_COLOR,
            width: 0.0,
            radius: Radius::new(5),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn subtract_button(
    _theme: &Theme,
    _status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    iced::widget::button::Style {
        background: Some(iced::Background::Color(SUBTRACT)),
        text_color: TEXT_COLOR,
        border: Border {
            color: BORDER_COLOR,
            width: 0.0,
            radius: Radius::new(5),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn thumbnail_card(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        text_color: Some(TEXT_COLOR),
        background: None,
        border: Border {
            color: BORDER_COLOR,
            width: 1.0,
            radius: Radius::new(0),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn thumbnail_card_highlight(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        text_color: Some(TEXT_COLOR),
        background: Some(iced::Background::Color(SELECTED_COLOR)),
        border: Border {
            color: SELECTED_COLOR,
            width: 1.0,
            radius: Radius::new(0),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn thumbnail_card_collected(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        text_color: Some(TEXT_COLOR),
        background: Some(iced::Background::Color(LIGHT_ORANGE)),
        border: Border {
            color: LIGHT_ORANGE,
            width: 1.0,
            radius: Radius::new(0),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn thumbnail_card_highlight_collected(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        text_color: Some(TEXT_COLOR),
        background: Some(iced::Background::Color(DARK_ORANGE)),
        border: Border {
            color: LIGHT_ORANGE,
            width: 1.0,
            radius: Radius::new(0),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn main_panel(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        text_color: None,
        background: Some(iced::Background::Color(LIGHT_GRAY)),
        border: Border {
            color: BORDER_COLOR,
            width: 0.0,
            radius: Radius::new(0),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn side_panel(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        text_color: None,
        background: Some(iced::Background::Color(DESELECTED_COLOR)),
        border: Border {
            color: BORDER_COLOR,
            width: 0.0,
            radius: Radius::new(0),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn progress_bar_on(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        text_color: None,
        background: Some(iced::Background::Color(PROGRESS_ON)),
        border: Border {
            color: BORDER_COLOR,
            width: 0.0,
            radius: Radius::new(0),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn progress_bar_off(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        text_color: None,
        background: Some(iced::Background::Color(PROGRESS_OFF)),
        border: Border {
            color: BORDER_COLOR,
            width: 0.0,
            radius: Radius::new(0),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn bottom_bar(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        text_color: Some(LIGHT_TEXT),
        background: Some(iced::Background::Color(BOTTOM_BAR)),
        border: Border {
            color: BORDER_COLOR,
            width: 0.0,
            radius: Radius::new(0),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn bottom_bar_warning(_theme: &Theme) -> iced::widget::container::Style {
    iced::widget::container::Style {
        text_color: Some(LIGHT_TEXT),
        background: Some(iced::Background::Color(DARK_ORANGE)),
        border: Border {
            color: BORDER_COLOR,
            width: 0.0,
            radius: Radius::new(0),
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

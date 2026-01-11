mod file;
mod loader;
mod search;
mod style;
use std::path::PathBuf;

use iced::alignment::Horizontal::Left;
use iced::alignment::Vertical;
use iced::event::Event;
use iced::keyboard;
use iced::keyboard::key::Named;
use iced::widget::button::Style;
// use iced::futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use iced::Fill;
use iced::FillPortion;
use iced::Shrink;
use iced::widget::image::{Allocation, Handle, Image, allocate};
use iced::widget::{
    Button, Column, Container, Grid, MouseArea, Row, Text, button, column, container, grid, row,
    scrollable, text,
};
use iced::window::Settings;
use iced::window::settings::PlatformSpecific;
use iced::{
    Alignment, Background, Border, Color, Element, Program, Subscription, Task, Theme, event,
    window,
};

use crate::file::{Dir, Img, Tag, TagIdx, default_dir};

// style constants for now
const THUMBNAIL_RES: u32 = 200;
const BOX_TEXT_HEIGHT: u32 = 20;
const SCROLLBAR_WIDTH: u32 = 10;
const LEFT_PANEL_WIDTH: u32 = 300;

///
/// The collection of images that the user has made.
///
struct Collection {
    /// The index within the entries list that is currently selected.
    selected: usize,
    /// The list of entries that are in the collection.
    entries: Vec<CollectionElement>,
}

/// An element within a collection.
struct CollectionElement {
    /// The corresponding index into the images list.
    /// Useful for looking at the tags on a collection element.
    img_idx: usize,
    /// The name of the file for this collection element.
    name: String,
    /// The allocation for this element's image. These images are full size,
    /// so we want to load them and keep their data stored here.
    alloc: Option<Allocation>, // option bc might be loading. How can we put it here?
}

struct ThumbnailProgress {
    current: u32,
    max: u32,
    shutdown_signal: tokio::sync::watch::Sender<bool>,
}
struct CollectionProgress {
    current: u32,
    max: u32,
}

/// State struct
struct Counter {
    panel: Panel,
    body_panel: BodyPanel,
    directory_list: Option<Vec<Dir>>,
    directory: PathBuf,
    file_list: Option<Vec<Img>>,
    file_filter_indices: Option<Vec<usize>>,
    search_content: String, // uhm, needs state reference here idk. Option?
    tags_list: Option<Vec<Tag>>,
    selected_file_idx: Option<usize>,
    error_message: Option<String>,
    collection: Option<Collection>,
    thumbnail_progress: Option<ThumbnailProgress>,
    collection_progress: Option<CollectionProgress>,
}
impl Default for Counter {
    fn default() -> Self {
        let default_directory = default_dir();
        // let file_list = file::image_list(&default_directory);

        // TODO: Can't do this here. Update needs the msg_stream. Where would
        // we plug it here?

        // match file_list.as_ref() {
        //     None => {}, // what ever.
        //     Some(list) => {
        //         let simple_list = list.iter().map(|entry| entry.path.clone()).collect();
        //         let (msg_stream, shutdown_tx) = loader::get_async_values(simple_list);
        //         shutdown_signal = Some(shutdown_tx);
        //     },
        // }

        let cur_counter = Self {
            panel: Panel::None,
            body_panel: BodyPanel::Explore,
            directory_list: file::directory_list(&default_directory),
            file_list: None,
            file_filter_indices: None,
            directory: default_directory,
            search_content: String::from(""),
            tags_list: None,
            selected_file_idx: None,
            error_message: None,
            collection: None,
            thumbnail_progress: None,
            collection_progress: None,
        };

        cur_counter
    }
}

/// State of the body.
#[derive(PartialEq, Eq)]
enum BodyPanel {
    /// Mode for looking at thumbnails of images.
    Explore,
    /// Mode for looking at images one-by-one.
    Collection,
}

/// State of the left panel.
#[derive(PartialEq, Eq)]
enum Panel {
    /// Indicates no panel present.
    None,
    /// Indicates the file panel present.
    File,
    /// Indicates the search panel present.
    Search,
    /// Indicates the tag panel present.
    Tag,
}

#[derive(Debug, Clone)]
pub enum Message {
    File,
    Search,
    Tag,
    SwitchDirectory(u16),       // index
    ImageLoaded(usize, Handle), // index into image array, with its handle
    AllImagesLoaded,
    AddTag(TagIdx),    // add an already existing tag to the selected file
    RemoveTag(TagIdx), // remove an already existing tag from the selected file
    AddInputTag,       // reference search
    SearchChanged(String),
    ImageSelected(usize), // index into image array, indicating selection
    SearchSend,
    AddPositiveSearchTerm(TagIdx),
    AddNegativeSearchTerm(TagIdx),
    Explore,
    Collection,
    ToggleIntoCollection(usize), // index into image array, indicating selection
    CollectionLeft,
    CollectionRight,
    CollectionImageAllocated(Allocation, usize), // index into image array
    CollectionImageFailed(usize),                // index into image array
    AddAllToCollection,
    ClearCollection,
    CloseApp(iced::window::Id), // window id? idk.
    KeyPressed(keyboard::Key),
}

/// Main methods for the state required by iced.
impl Counter {
    fn update(&mut self, message: Message) -> Task<Message> {
        // destroy error message
        self.error_message = None;
        match message {
            Message::File => {
                self.panel = match self.panel {
                    Panel::File => Panel::None,
                    _ => Panel::File,
                };
                Task::none()
            }
            Message::Search => {
                self.panel = match self.panel {
                    Panel::Search => Panel::None,
                    _ => Panel::Search,
                };
                Task::none()
            }
            Message::Tag => {
                self.panel = match self.panel {
                    Panel::Tag => Panel::None,
                    _ => Panel::Tag,
                };
                Task::none()
            }
            Message::SwitchDirectory(idx) => self.go_to_dir(Some(idx as usize)),
            Message::ImageLoaded(idx, handle) => {
                if let Some(i) = self.thumbnail_progress.as_mut() {
                    i.current += 1;
                }
                match self.file_list.as_mut() {
                    Some(list) => {
                        if idx < list.len() {
                            list[idx].thumbnail_handle = Some(handle);
                        }
                        Task::none()
                    }
                    None => Task::none(),
                }
            }
            Message::AllImagesLoaded => {
                self.thumbnail_progress = None;
                Task::none()
            }
            Message::AddTag(tidx) => {
                // existing tag
                self.add_tag_to_selected(tidx);
                Task::none()
            }
            Message::RemoveTag(tidx) => {
                // existing tag

                // ensure file selected
                let file_idx = match self.get_selected_index() {
                    None => return Task::none(), // no file selected tf???
                    Some(v) => v,
                };

                // now, get the img list
                let file_list_obj = match self.file_list.as_mut() {
                    None => return Task::none(),
                    Some(list) => list,
                };

                // get the actual image
                let img = &mut file_list_obj[file_idx];
                match img.tags.as_mut() {
                    None => return Task::none(),
                    Some(tlist) => {
                        // need to check if the tag exists in here.
                        let idx = match tlist.iter().position(|x| *x == tidx) {
                            None => return Task::none(),
                            Some(id) => id,
                        };

                        // remove it from the list
                        tlist.swap_remove(idx);

                        if tlist.len() == 0 {
                            img.tags = None; // no tags there anymore!
                        }
                    }
                }

                // now, remove a reference from the tag
                // get tag list
                let tag_list_obj = match self.tags_list.as_mut() {
                    None => return Task::none(), // no tasks available...?
                    Some(list) => list,
                };

                // get object in list
                let tag = &mut tag_list_obj[tidx as usize];

                if tag.refs == 0 {
                    self.error_message = Some(String::from(
                        "Warning! We removed a tag that had zero references. Something is off here.",
                    ));
                    return Task::none();
                    // this might indicate a bug, or a user who spammed the minus button.
                }
                tag.refs -= 1;

                // so, we only add the tag and increment ref if both succeed.

                Task::none()
            }
            Message::AddInputTag => {
                // TODO: have checks on the self.search_content, make sure validity
                if !is_valid_tag(&self.search_content) {
                    self.error_message = Some(String::from("Invalid tag name."));
                    return Task::none();
                }

                // TODO: move the AddTag into its own function, then have this
                // call that after we make the brand new tag
                let tags_list = match self.tags_list.as_mut() {
                    None => {
                        self.tags_list = Some(Vec::new());
                        self.tags_list.as_mut().unwrap()
                    }
                    Some(l) => l,
                };

                // get index of tag if already exists, or none otherwise
                let idx_opt = tags_list
                    .iter()
                    .position(|tag| tag.name == self.search_content);

                match idx_opt {
                    None => {
                        let next_idx = match tags_list.last() {
                            Some(i) => i.idx + 1,
                            None => 0,
                        };
                        tags_list.push(Tag {
                            name: self.search_content.clone(),
                            idx: next_idx,
                            refs: 0,
                        }); // make with 0, AddTag will handle that
                        self.search_content = String::from("");
                        self.add_tag_to_selected(next_idx);
                    }
                    Some(i) => self.add_tag_to_selected(i as u32),
                }

                Task::none()
            }
            Message::SearchChanged(str) => {
                self.search_content = str;
                Task::none()
            }
            Message::ImageSelected(idx) => {
                // here, we actually want to match with selected_file_idx
                // this message is the one that lets us select/deselect
                match self.selected_file_idx {
                    Some(i) if i == idx => self.selected_file_idx = None,
                    _ => self.selected_file_idx = Some(idx),
                }
                Task::none()
            }
            Message::SearchSend => {
                println!("Not done here.");
                let tags_list = match self.tags_list.as_ref() {
                    Some(v) => v,
                    None => {
                        self.file_filter_indices = None;
                        self.error_message = Some(String::from("No tags to search for."));
                        return Task::none();
                    }
                };
                let images = match self.file_list.as_ref() {
                    Some(v) => v,
                    None => {
                        self.file_filter_indices = None;
                        self.error_message = Some(String::from("No files to look at."));
                        return Task::none();
                    }
                };

                let filter_indices = match search::filter_to_string(
                    self.search_content.clone(),
                    tags_list,
                    images,
                ) {
                    Ok(v) => v,
                    Err(s) => {
                        //whatever
                        self.file_filter_indices = None;
                        self.error_message = Some(s);
                        return Task::none();
                    }
                };
                println!("Got these filter indices: {:?}", filter_indices);

                self.file_filter_indices = Some(filter_indices);

                // need to tell the image loader to load smth else now
                if let Some(i) = self.thumbnail_progress.take() {
                    let _ = i.shutdown_signal.send(true); // don't care if fails.
                    // if it fails that's good, we want to shut down.
                }

                // calculate the ones that need to be loaded
                let imgs_to_load: Vec<(usize, PathBuf)> = match self.file_list.as_ref() {
                    Some(file_list) => self
                        .file_filter_indices
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|x| (x, &file_list[*x]))
                        .filter(|x| x.1.thumbnail_handle == None)
                        .map(|x| (*x.0, x.1.path.clone()))
                        .collect(),
                    None => return Task::none(),
                };
                let num_to_load: u32 = imgs_to_load.len() as u32;

                // tell it to work on these kitties
                let (msg_stream, shutdown_tx) =
                    loader::get_async_values(imgs_to_load, THUMBNAIL_RES);
                // TODO there is some unfortunate stuff that MIGHT happen. But the extent of this misfortune
                // is simply that the progress bar is slightly behind (by like 1) and will never catch up.
                self.thumbnail_progress = Some(ThumbnailProgress {
                    current: 0,
                    max: num_to_load,
                    shutdown_signal: shutdown_tx,
                });
                Task::run(msg_stream, |x| x)
            }
            Message::AddPositiveSearchTerm(tidx) => {
                if let Some(l) = self.tags_list.as_ref() {
                    // TODO actually, I hate all of this.
                    // I would prefer that the search bar does some
                    // sort of auto complete.
                    self.search_content += " ";
                    self.search_content += &l[tidx as usize].name;
                }
                return Task::none();
            }
            Message::AddNegativeSearchTerm(tidx) => {
                if let Some(l) = self.tags_list.as_ref() {
                    self.search_content += " -";
                    self.search_content += &l[tidx as usize].name;
                }
                return Task::none();
            }
            Message::Collection => {
                self.body_panel = BodyPanel::Collection;
                Task::none()
            }
            Message::ToggleIntoCollection(idx) => {
                match self.collection.as_mut() {
                    None => {
                        // make new

                        let img: &Img = match self.file_list.as_ref() {
                            None => return Task::none(), // ????
                            Some(list) => &list[idx],
                        };

                        let entry_name = img.name.clone();

                        let first_elt = CollectionElement {
                            img_idx: idx,
                            name: entry_name, // need to load it
                            alloc: None,
                        };

                        self.collection = Some(Collection {
                            selected: 0,
                            entries: vec![first_elt],
                        });

                        // progress
                        self.collection_progress = Some(CollectionProgress { current: 0, max: 1 });

                        let path_clone = img.path.clone();
                        let handle = Handle::from_path(path_clone);

                        allocate(handle).map(move |res| match res {
                            Ok(alloc) => Message::CollectionImageAllocated(alloc, idx),
                            Err(_) => Message::CollectionImageFailed(idx),
                        })
                    }
                    Some(col) => {
                        // check if add or subtract

                        // TODO .position is inefficient here. Really, we want
                        // to search from the END, since we can do it on O(1)
                        // because users will never ever add a ton of images to
                        // the collection immediately. there will be time.
                        match col.entries.iter().position(|x| x.img_idx == idx) {
                            Some(i) => {
                                // subtract
                                // remove at the given index
                                // REMEMBER: Since we are removing, might have to make None.
                                col.entries.remove(i);
                                let new_len = col.entries.len();
                                if new_len == 0 {
                                    self.collection = None;
                                } else if col.selected >= new_len {
                                    col.selected = new_len - 1;
                                }
                                Task::none()
                            }
                            None => {
                                // add the entry

                                let img: &Img = match self.file_list.as_ref() {
                                    None => return Task::none(), // ????
                                    Some(list) => &list[idx],
                                };

                                let entry_name = img.name.clone();

                                let new_element = CollectionElement {
                                    img_idx: idx,
                                    name: entry_name, // need to load it
                                    alloc: None,
                                };

                                col.entries.push(new_element);

                                // progress
                                match self.collection_progress.as_mut() {
                                    None => {
                                        self.collection_progress =
                                            Some(CollectionProgress { current: 0, max: 1 })
                                    }
                                    Some(p) => {
                                        p.max += 1;
                                    }
                                }

                                let path_clone = img.path.clone();
                                let handle = Handle::from_path(path_clone);

                                allocate(handle).map(move |res| match res {
                                    Ok(alloc) => Message::CollectionImageAllocated(alloc, idx),
                                    Err(_) => Message::CollectionImageFailed(idx),
                                })
                            }
                        }
                    }
                }
            }
            Message::CollectionLeft => {
                match self.collection.as_mut() {
                    None => Task::none(), // can't do anything
                    Some(c) => {
                        if c.selected > 0 {
                            c.selected -= 1;
                        }
                        Task::none()
                    }
                }
            }
            Message::CollectionRight => {
                match self.collection.as_mut() {
                    None => Task::none(), // can't do anything
                    Some(c) => {
                        if c.selected < c.entries.len() - 1 {
                            c.selected += 1;
                        }
                        Task::none()
                    }
                }
            }
            Message::CollectionImageAllocated(allocation, idx) => {
                if let Some(col) = self.collection.as_mut() {
                    if let Some(entry) = col.entries.iter_mut().rev().find(|elt| elt.img_idx == idx)
                    {
                        entry.alloc = Some(allocation);
                        if let Some(p) = self.collection_progress.as_mut() {
                            p.current += 1;
                            if p.current >= p.max {
                                self.collection_progress = None; // done
                            }
                        }
                    }
                    // ignore if missing
                }
                // ignore if missing
                Task::none()
            }
            Message::CollectionImageFailed(_) => {
                println!("Allocating the collection image failed. Not sure what to do!");
                Task::none()
            }
            Message::Explore => {
                self.body_panel = BodyPanel::Explore;
                Task::none()
            }
            Message::AddAllToCollection => {
                // add all images (respecting filter) to the collection
                let images_list = match self.file_list.as_ref() {
                    Some(l) => l,
                    None => return Task::none(), // nothing to add, do nothing
                };

                let images_to_add: Vec<(usize, &Img)> = match self.file_filter_indices.as_ref() {
                    None => images_list.iter().enumerate().map(|x| x).collect(), // todo uhmmm?? is this ok?
                    Some(i) => i.iter().map(|idx| (*idx, &images_list[*idx])).collect(),
                };

                if images_to_add.len() == 0 {
                    return Task::none();
                }

                // ok, so we definitely need to add some.
                // make collection if non-existent, bc we will add something
                if let None = self.collection {
                    self.collection = Some(Collection {
                        selected: 0,
                        entries: Vec::new(),
                    })
                }
                let col = self.collection.as_mut().unwrap();

                let mut tasks: Vec<Task<Message>> = Vec::new();
                let num_to_load: u32 = images_to_add.len() as u32;

                for (idx, image) in images_to_add {
                    let new_element = CollectionElement {
                        img_idx: idx,
                        name: image.name.clone(),
                        alloc: None,
                    };

                    col.entries.push(new_element);

                    let path_clone = image.path.clone();
                    let handle = Handle::from_path(path_clone);

                    tasks.push(allocate(handle).map(move |res| match res {
                        Ok(alloc) => Message::CollectionImageAllocated(alloc, idx),
                        Err(_) => Message::CollectionImageFailed(idx),
                    }));
                }

                // progress
                match self.collection_progress.as_mut() {
                    None => {
                        self.collection_progress = Some(CollectionProgress {
                            current: 0,
                            max: num_to_load,
                        })
                    }
                    Some(p) => {
                        p.max += num_to_load;
                    }
                }

                Task::batch(tasks) // run all the allocs
            }
            Message::ClearCollection => {
                // just clear it!
                self.collection = None;
                Task::none()
            }
            Message::CloseApp(id) => {
                self.compress_and_save_to_file();
                iced::window::close(id)
            }
            Message::KeyPressed(key) => {
                match key {
                    keyboard::Key::Named(Named::ArrowLeft) => {
                        Task::done(Message::CollectionLeft)
                    },
                    keyboard::Key::Named(Named::ArrowRight) => {
                        Task::done(Message::CollectionRight)
                    },
                    _ => todo!(),
                }
            },
        }
    }

    fn view<'a>(&'a self) -> Element<'a, Message> {
        // TODO we can maybe store this in self.
        let dir_name: String = match self.directory.as_os_str().to_str() {
            None => String::from("?"),
            Some(i) => i.to_owned(),
        };

        let collection_name: String = match self.collection.as_ref() {
            None => String::from("Collection"),
            Some(i) => format!("Collection ({})", i.entries.len()),
        };

        let top_bar: Container<Message> = container(
            row![
                button("File")
                    .on_press(Message::File)
                    .style(if self.panel == Panel::File {
                        crate::style::selected_button
                    } else {
                        crate::style::deselected_button
                    }),
                button("Search")
                    .on_press(Message::Search)
                    .style(if self.panel == Panel::Search {
                        crate::style::selected_button
                    } else {
                        crate::style::deselected_button
                    }),
                button("Tag")
                    .on_press(Message::Tag)
                    .style(if self.panel == Panel::Tag {
                        crate::style::selected_button
                    } else {
                        crate::style::deselected_button
                    }),
                text(dir_name).width(Fill).center(),
            ]
            .align_y(iced::Alignment::Center)
            .spacing(3)
            .padding(5),
        )
        .width(Fill)
        .padding(5);
        let left_panel;

        {
            let left_panel_width: u32 = match self.panel {
                Panel::None => 0,
                _ => 300,
            };

            let left_contents: Column<Message> = match self.panel {
                Panel::File => self.get_file_panel(),
                Panel::Search => self.get_search_panel(),
                Panel::Tag => self.get_tag_panel(),
                _ => column![],
            };

            left_panel = container(scrollable(left_contents))
                .style(style::side_panel)
                .width(left_panel_width)
                .height(Fill)
                .padding(3);
        }

        let selected_img_txt: String = match self.get_selected_index() {
            Some(i) => match self.file_list.as_ref() {
                Some(list) => truncate(&list[i].name[..], 64),
                None => String::from("No file selected."),
            },
            None => String::from("No file selected."),
        };

        let body_bar: Row<Message> = row![
            button("Explore").on_press(Message::Explore).style(
                if self.body_panel == BodyPanel::Explore {
                    crate::style::selected_button
                } else {
                    crate::style::deselected_button
                }
            ),
            button(text(collection_name))
                .on_press(Message::Collection)
                .style(if self.body_panel == BodyPanel::Collection {
                    crate::style::selected_button
                } else {
                    crate::style::deselected_button
                }),
            container(text(selected_img_txt).width(Fill).center())
                .width(Fill)
                .height(Fill)
                .clip(true),
            button("Add all to Collection")
                .on_press(Message::AddAllToCollection)
                .style(style::standard_button),
            button("Clear Collection")
                .on_press(Message::ClearCollection)
                .style(style::standard_button)
        ]
        .align_y(iced::Alignment::Center)
        .spacing(3)
        .padding(5)
        .width(Fill)
        .height(35);

        let body_content: Container<Message> = match self.body_panel {
            BodyPanel::Collection => self.get_collection_body(),
            BodyPanel::Explore => {
                container(scrollable(self.view_image_grid()).width(Fill).height(Fill))
                    .width(Fill)
                    .height(Fill)
            }
        };

        let body = column![body_bar, body_content.padding(5)]
            .width(Fill)
            .height(Fill)
            .spacing(5);

        let main_container = container(
            row![
                left_panel,
                container(body)
                    .width(Fill)
                    .height(Fill)
                    .style(style::main_panel)
            ]
            .spacing(5),
        )
        .width(Fill)
        .height(Fill);

        let full_app: Element<'a, Message> =
            container(column![top_bar, main_container, self.view_bottom_bar()]).into();

        full_app
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            window::events().filter_map(|(id, event)| match event {
                window::Event::CloseRequested => Some(Message::CloseApp(id)),
                _ => None,
            }),
            iced::event::listen_with(|event, _, _| match event {
                Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                    Some(Message::KeyPressed(key))
                }
                _ => None,
            }),
        ])
    }
}

/// Helper methods.
impl Counter {
    /// Adds the tag with index tidx to the selected file.
    fn add_tag_to_selected(&mut self, tidx: TagIdx) {
        // ensure file selected
        let file_idx = match self.get_selected_index() {
            None => return, // no file selected tf???
            Some(v) => v,
        };

        // ok, get the list
        let tag_list_obj = match self.tags_list.as_mut() {
            None => {
                return;
            } // no tasks available...?
            Some(list) => list,
        };

        // get object in list
        let tag = &mut tag_list_obj[tidx as usize];

        // now, get the img list
        let file_list_obj = match self.file_list.as_mut() {
            None => {
                return;
            }
            Some(list) => list,
        };

        // get the actual image
        let img = &mut file_list_obj[file_idx];
        match img.tags.as_mut() {
            None => {
                let mut tag_vec_temp = Vec::new();
                tag_vec_temp.push(tidx);
                img.tags = Some(tag_vec_temp);
                tag.refs += 1;
            }
            Some(tlist) => {
                // need to check if the tag exists in here.
                if !tlist.contains(&tidx) {
                    tlist.push(tidx);
                    // now, only if we get HERE should we increment tag refs
                    tag.refs += 1;
                }
            }
        }

        // so, we only add the tag and increment ref if both succeed.
    }

    pub fn get_file_panel<'a>(&'a self) -> Column<'a, Message> {
        let mut col: Column<Message> = Column::new();
        col = col.push("Files:");
        if let None = self.directory_list {
            col = col.push("Failed to get directory list.");
            return col;
        }
        let list = self.directory_list.as_ref().unwrap();
        let mut i = 0;
        for elt in list {
            col = col.push(
                button(text(&elt.name).width(Fill))
                    .on_press(Message::SwitchDirectory(i))
                    .style(style::standard_button),
            );
            i += 1;
        }
        col.spacing(4).padding(4)
    }

    pub fn view_image_grid<'a>(&'a self) -> Grid<'a, Message> {
        // column of rows
        // first, get the images
        if let None = self.file_list {
            return Grid::new().push("Failed to get files in this directory.");
        }
        let list = self.file_list.as_ref().unwrap();
        if list.len() == 0 {
            return Grid::new().push("There's no images here. Use the \"File\" tab to navigate to a directory that has images.");
        }
        let filtered_img_list: Vec<&Img> = match self.file_filter_indices.as_ref() {
            Some(l) => l.iter().map(|x| &list[*x]).collect::<Vec<&Img>>(),
            None => list.iter().map(|x| x).collect::<Vec<&Img>>(),
        };
        // need to filter
        let grid =
            filtered_img_list
                .iter()
                .fold(Grid::new().spacing(0), |grid: Grid<Message>, entry| {
                    let selected = match self.selected_file_idx.as_ref() {
                        Some(idx) if *idx == entry.idx => true,
                        _ => false
                    };

                    grid.push(
                        view_thumbnail_card(entry, selected)
                    )
                });

        grid.fluid(200)

        // let iter = filtered_img_list.chunks_exact(IMG_PER_ROW);
        // let remainder = iter.remainder();

        // // wow this stinks!
        // let mut cur_idx = 0;
        // ok let's try grid

        // let mut col: Column<Message> = iter.fold(Column::new(), |col: Column<Message>,entries| {
        //     let our_row = entries.iter().fold(Row::new().width(Fill).spacing(10), |row: Row<Message>, entry| { // TODO const?!?!
        //         let container_content: Element<'_, Message> = match entry.thumbnail_handle.as_ref() {
        //             Some(handle) => {
        //                 MouseArea::new(
        //                     Image::new(
        //                         handle.clone()
        //                     ).width(THUMBNAIL_RES).height(THUMBNAIL_RES).content_fit(iced::ContentFit::Cover)
        //                 ).on_press(Message::ImageSelected(cur_idx)).on_right_press(Message::ToggleIntoCollection(cur_idx)).into()
        //             },
        //             None => Text::new("Loading...").into(),
        //         };
        //         cur_idx += 1;
        //         row.push(

        //             container(
        //                 column![
        //                     container(
        //                         container_content
        //                     ).height(Fill).width(Fill),
        //                     text(&entry.name).height(BOX_TEXT_HEIGHT).width(Fill),

        //                 ].width(Fill).height(Fill)
        //             ).width(Fill).height(BOX_HEIGHT).style(container::bordered_box)
        //         )
        //     });
        //     col.push(our_row)

        // }).width(Fill).height(Fill).spacing(10); // TODO const??

        // // TODO remainder is all weird urf
        // // TODO repeated code in remainder section... how to deal with?
        // // TODO remainder is just gonna be forgotten about for now.
        // if remainder.len() > 0 {
        //     let remainder_row = remainder.iter().fold(Row::new().width(Fill).spacing(10),|row: Row<Message>, entry| { // TODO const
        //         let container_content: Element<'_, Message> = match entry.thumbnail_handle.as_ref() {
        //             Some(handle) => {
        //                 MouseArea::new(
        //                     Image::new(
        //                         handle.clone()
        //                     ).width(Fill).height(Fill).content_fit(iced::ContentFit::Cover)
        //                 ).on_press(Message::ImageSelected(cur_idx)).on_right_press(Message::ToggleIntoCollection(cur_idx)).into()
        //             },
        //             None => Text::new("Loading...").into(),
        //         };
        //         cur_idx += 1;
        //         row.push(

        //             container(
        //                 column![
        //                     container(
        //                         container_content
        //                     ).height(Fill).width(Fill),
        //                     text(&entry.name).height(BOX_TEXT_HEIGHT).width(Fill),

        //                 ].width(Fill).height(Fill)
        //             ).width(Fill).height(BOX_HEIGHT).style(container::bordered_box)
        //         )
        //     });
        //     col = col.push(remainder_row);
        // }

        //col
    }

    pub fn get_tag_panel<'a>(&'a self) -> Column<'a, Message> {
        let existing_tags_col = Column::new();

        let selected_idx = self.get_selected_index();

        if let None = selected_idx {
            return existing_tags_col
                .push(text("Select a file to add tags."))
                .padding(10);
        }

        if let None = self.tags_list {
            return existing_tags_col
                .push(text("Add your first tag."))
                .padding(10);
        }

        let list = self.tags_list.as_ref().unwrap();

        let existing_tags_col = list
            .iter()
            .fold(existing_tags_col, |col: Column<Message>, tag| {
                col.push(row![
                    text(tag.name.clone()).width(Fill),
                    button("+")
                        .on_press(Message::AddTag(tag.idx))
                        .style(style::add_button),
                    button("-")
                        .on_press(Message::RemoveTag(tag.idx))
                        .style(style::subtract_button)
                ])
            })
            .padding(10);

        let tags_on_image_container = self.get_tags_on_image_container(Some(|tidx| {
            vec![
                button("-")
                    .on_press(Message::RemoveTag(tidx))
                    .style(style::subtract_button),
            ]
        }));

        let top_row = row![
            iced::widget::text_input("Enter tag...", &self.search_content)
                .width(Fill)
                .on_input(|s| Message::SearchChanged(s)),
            button("+")
                .on_press(Message::AddInputTag)
                .style(style::add_button)
        ]
        .width(Fill);

        let outer_col = column![
            top_row,
            existing_tags_col,
            iced::widget::rule::horizontal(1),
            tags_on_image_container
        ];

        outer_col
    }

    pub fn get_search_panel<'a>(&'a self) -> Column<'a, Message> {
        let top_row = row![
            iced::widget::text_input("Search...", &self.search_content)
                .width(Fill)
                .on_input(|s| Message::SearchChanged(s)),
            button("Go")
                .on_press(Message::SearchSend)
                .style(style::add_button)
        ]
        .width(Fill);

        let mut col = Column::new().push(top_row);

        col = col.push(self.get_tags_on_image_container(Some(|tidx| {
            vec![
                button("+")
                    .on_press(Message::AddPositiveSearchTerm(tidx))
                    .style(style::add_button),
                button("-")
                    .on_press(Message::AddNegativeSearchTerm(tidx))
                    .style(style::subtract_button),
            ]
        })));

        col
    }

    ///
    /// Gets the body for viewing a collection of images.
    /// A collection of images is something the user decides. They can add
    /// images to the collection, and also subtract these images. The collection
    /// view allows users to see the images at maximal resolution and cycle
    /// through with buttons (or later arrow keys)
    fn get_collection_body<'a>(&'a self) -> Container<'a, Message> {
        let (collection_entries, selected_idx) = match self.collection.as_ref() {
            Some(i) => (&i.entries, i.selected),
            None => return container(text("Make a collection first!")),
        };

        let col_entry = &collection_entries[selected_idx];

        let main_elt: Element<'a, Message> = match col_entry.alloc.as_ref() {
            Some(alloc) => Image::new(alloc.handle())
                .width(Fill)
                .height(Fill)
                .content_fit(iced::ContentFit::ScaleDown)
                .into(),
            None => text("Loading...").center().into(),
        };

        let center_stack = column![
            button("Remove from Collection")
                .on_press(Message::ToggleIntoCollection(col_entry.img_idx))
                .height(Shrink)
                .style(style::subtract_button),
            main_elt
        ];
        let entire_panel = row![
            button("<")
                .on_press(Message::CollectionLeft)
                .style(style::standard_button),
            center_stack.height(Fill).width(Fill),
            button(">")
                .on_press(Message::CollectionRight)
                .style(style::standard_button)
        ]
        .width(Fill)
        .height(Fill)
        .align_y(Vertical::Center);

        container(entire_panel)
            .width(Fill)
            .height(Fill)
            .style(container::bordered_box)
    }
    ///
    /// Look. I know it's a hellish type.
    /// You have the OPTION to generate buttons per tag.
    /// You can have as many buttons that you want, which will be put in a row format.
    fn get_tags_on_image_container<'a>(
        &'a self,
        button_generator: Option<fn(TagIdx) -> Vec<Button<'a, Message>>>,
    ) -> Element<'a, Message> {
        let mut col = Column::new();
        col = col.push(text("Tags on this image"));

        let contain = |c: Column<'a, Message>| -> Element<'a, Message> {
            container(c.width(Fill).spacing(3).padding(10)).into()
        };

        let contain_empty = |c: Column<'a, Message>| -> Element<'a, Message> {
            contain(c.push(text("None")).padding(10))
        };

        let selected_idx = self.get_selected_index();

        if let None = selected_idx {
            return contain_empty(col);
        }

        if let None = self.file_list {
            return contain_empty(col); // shouldn't be possible given above is None, but good to check i guess.
        }

        if let None = self.tags_list {
            return contain_empty(col);
        }

        let selected_file: &Img = &self.file_list.as_ref().unwrap()[selected_idx.unwrap()];

        if let None = selected_file.tags {
            return contain_empty(col);
        }

        // GREAT. So things exist!

        let tags_global_list = self.tags_list.as_ref().unwrap();
        let tags_file_list = selected_file.tags.as_ref().unwrap();

        for tidx in tags_file_list {
            let exp_tag = &tags_global_list[*tidx as usize];
            match button_generator.as_ref() {
                None => col = col.push(text(&exp_tag.name).width(Fill)),
                Some(mapper) => {
                    let mut row: Row<Message> = row![text(&exp_tag.name).width(Fill)];
                    let buttons = mapper(*tidx);
                    for btn in buttons {
                        row = row.push(btn);
                    }
                    col = col.push(row);
                }
            }
        }

        contain(col)
    }

    fn get_dir_from(&self, idx: Option<usize>) -> PathBuf {
        let list = match self.directory_list.as_ref() {
            None => return file::default_dir(),
            Some(l) => l,
        };
        match idx {
            None => file::default_dir(),
            Some(i) => list[i].path.clone(),
        }
    }

    ///
    /// Tries to save to the metadata file in the current directory, if there is
    /// data to save. Since we are saving, we also are compressing the data,
    /// hence mutability.
    fn compress_and_save_to_file(&mut self) {
        // before the swap, let's record our data.
        if let Some(tags) = self.tags_list.as_mut() {
            // have data to record
            if let Some(images) = self.file_list.as_mut() {
                let metadata_path: PathBuf = self.directory.join(file::METADATA_NAME);
                // before we write, let's compress tags
                file::compress_tags(tags, images);
                if !file::write_metadata(&metadata_path, tags, images) {
                    self.error_message = Some(String::from("Could not write metadata to file."));
                }
            }
        }
    }

    ///
    /// Forces swap to new directory
    /// Does not save current info to file.
    fn load_values_of_current_dir(&mut self) -> Task<Message> {
        self.tags_list = None;
        self.file_filter_indices = None;
        self.collection = None;
        self.selected_file_idx = None;

        // send shutdown signal to other thread loading thumbnails
        if let Some(i) = self.thumbnail_progress.take() {
            let _ = i.shutdown_signal.send(true); // don't care if fails.
            // if it fails that's good cuz we want it to shut down.
        }

        self.directory_list = file::directory_list(&self.directory);

        match file::image_list(&self.directory) {
            None => {
                self.file_list = None;
                Task::none()
            }
            Some(list) => {
                let simple_list: Vec<(usize, PathBuf)> = list
                    .iter()
                    .enumerate()
                    .map(|entry| (entry.0, entry.1.path.clone()))
                    .collect();
                self.file_list = Some(list);
                // read metadata
                match file::try_get_metadata_path(&self.directory) {
                    Some(metadata_path) => {
                        self.tags_list =
                            file::read_metadata(&metadata_path, self.file_list.as_mut().unwrap())
                    }
                    None => {}
                }
                let num_to_load = simple_list.len() as u32;
                let (msg_stream, shutdown_tx) =
                    loader::get_async_values(simple_list, THUMBNAIL_RES);
                self.thumbnail_progress = Some(ThumbnailProgress {
                    current: 0,
                    max: num_to_load,
                    shutdown_signal: shutdown_tx,
                });
                Task::run(msg_stream, |x| x)
            }
        }
    }
    ///
    /// Goes to a directory, given an index to the directory.
    /// Also saves the current info.
    fn go_to_dir(&mut self, idx: Option<usize>) -> Task<Message> {
        // first, try to save and compress.
        self.compress_and_save_to_file();

        self.directory = self.get_dir_from(idx);

        self.load_values_of_current_dir()
    }

    /// Gets the index of the file selected in the files list.
    /// Note that this is important because we could have selected an image in
    /// explore, or we could have selected an image in a collection.
    /// Returns None if no selection.
    fn get_selected_index(&self) -> Option<usize> {
        match self.body_panel {
            BodyPanel::Explore => match self.selected_file_idx {
                None => None,
                Some(idx) => Some(idx),
            },
            BodyPanel::Collection => match self.collection.as_ref() {
                None => None,
                Some(col) => {
                    let entry = &col.entries[col.selected];
                    Some(entry.img_idx)
                }
            },
        }
    }

    fn view_bottom_bar<'a>(&'a self) -> Container<'a, Message> {
        let basic_bar = || -> Container<'a, Message> {
            container(text("Ready").align_y(Alignment::Center))
                .style(style::bottom_bar)
                .padding(10)
        };

        let progress = |cur: u32, max: u32| -> Container<'a, Message> {
            container(
                row![
                    text(format!("{}/{}", cur, max))
                        .width(Shrink)
                        .height(Fill)
                        .align_y(Alignment::Center),
                    progress_bar(cur, max).width(Fill).height(Fill)
                ]
                .width(Fill)
                .height(Fill)
                .padding(10)
                .spacing(5),
            )
            .style(style::bottom_bar)
        };

        let contents: Container<'a, Message> = if let Some(msg) = self.error_message.as_ref() {
            container(text(msg.clone()))
                .style(style::bottom_bar_warning)
                .padding(10)
        } else {
            match self.body_panel {
                BodyPanel::Collection => match self.collection_progress.as_ref() {
                    Some(prog) if prog.max > 1 => progress(prog.current, prog.max),
                    _ => basic_bar(),
                },
                BodyPanel::Explore => match self.thumbnail_progress.as_ref() {
                    Some(prog) if prog.max > 1 => progress(prog.current, prog.max),
                    _ => basic_bar(),
                },
            }
        };
        contents.width(Fill).height(50)
    }
}

fn progress_bar<'a>(amt: u32, total: u32) -> Row<'a, Message> {
    let scaled_amt: u16;
    let scaled_total: u16;
    if total > u16::MAX as u32 {
        let proportion: f32 = (amt as f32) / (total as f32);
        scaled_amt = (proportion * amt as f32) as u16;
        scaled_total = u16::MAX;
    } else {
        scaled_amt = amt as u16;
        scaled_total = total as u16;
    };
    row![
        Container::new(iced::widget::Space::new())
            .width(FillPortion(scaled_amt))
            .height(Fill)
            .style(style::progress_bar_on),
        Container::new(iced::widget::Space::new())
            .width(FillPortion(scaled_total - scaled_amt))
            .height(Fill)
            .style(style::progress_bar_off)
    ]
}

fn view_thumbnail_card(image: &Img, selected: bool) -> Container<Message> {
    let container_content: Element<'_, Message> =
        match image.thumbnail_handle.as_ref() {
            Some(handle) => MouseArea::new(
                Image::new(handle.clone())
                    .width(THUMBNAIL_RES)
                    .height(THUMBNAIL_RES)
                    .content_fit(iced::ContentFit::Cover),
            )
            .on_press(Message::ImageSelected(image.idx))
            .on_right_press(Message::ToggleIntoCollection(image.idx))
            .into(),
            None => Text::new("Loading...").into(),
        };
    // determine whether to highlight

    let selection_style = match selected {
        true => style::thumbnail_card_highlight,
        false => style::thumbnail_card,
    };

    container(
        column![
            container(container_content).height(Fill).width(Fill),
            container(text(&image.name).height(BOX_TEXT_HEIGHT).width(Fill))
                .clip(true),
        ]
        .width(Fill)
        .height(Fill)
        .spacing(4)
        .padding(4),
    )
    .style(selection_style) // height thumbnail res?


}

pub fn is_valid_tag(name: &str) -> bool {
    if name.len() == 0 || !name.is_ascii() {
        return false;
    }
    for c in name.chars() {
        match c {
            '0'..='9' | 'a'..='z' | 'A'..='Z' | '_' => {}
            _ => return false,
        }
    }
    return true;
}

///
/// max_chars must be >4. Don't use tiny values.
fn truncate(s: &str, max_chars: usize) -> String {
    if s.len() < max_chars {
        return s.to_owned();
    }

    let mut full_str: String = s[0..(max_chars - 4)].to_owned();
    full_str.push_str("...");
    full_str
}

fn window_settings() -> window::Settings {
    Settings {
        size: iced::Size::new(800.0, 600.0), // want to initially be full screen
        maximized: true,
        fullscreen: false,
        position: window::Position::Default, // want just default i guess. not sure what this means. center of screen
        min_size: Some(iced::Size::new(400.0, 0.0)), // min is 400 width, height i dont care
        max_size: None,                      // max doesn't matter, as large as desired
        visible: true,
        resizable: true,
        closeable: true,
        minimizable: true,
        decorations: true,
        transparent: false,
        blur: false,
        level: window::Level::Normal, // i dont know what to do for this
        icon: None,                   // i dont have an icon yet. can i use a default?
        platform_specific: PlatformSpecific::default(), // not sure what to put here. can i use default?
        exit_on_close_request: false,
    }
}

#[tokio::main]
async fn main() {
    let _ = iced::application(
        || {
            let mut counter = Counter::default();
            let tasks = counter.load_values_of_current_dir();
            (counter, tasks)
        },
        Counter::update,
        Counter::view,
    )
    .window(window_settings())
    .subscription(Counter::subscription)
    .run();

    // let _ = iced::run(Counter::update, Counter::view);
}

mod file;
mod loader;
mod search;
mod style;
use std::path::PathBuf;

use iced::alignment::Vertical;
// use iced::futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use iced::widget::{Button, Column, Container, MouseArea, Row, Text, button, column, container, row, scrollable, text, grid, Grid};
use iced::widget::image::{Allocation, Handle, Image, allocate};
use iced::{Alignment, Background, Border, Color, Element, Task, Theme};
use iced::Fill;

use crate::file::{Dir, Img, Tag, TagIdx, default_dir};

// style constants for now
const THUMBNAIL_RES: u32 = 200;
const BOX_TEXT_HEIGHT: u32 = 20;

struct Collection {
    selected: usize,
    entries: Vec<CollectionElement>,
}

struct CollectionElement {
    img_idx: usize,
    name: String,
    alloc: Option<Allocation>, // option bc might be loading. How can we put it here?
}

struct Counter{
    panel: Panel,
    directory_list: Option<Vec<Dir>>,
    directory: PathBuf,
    file_list: Option<Vec<Img>>,
    file_filter_indices: Option<Vec<usize>>,
    shutdown_signal: Option<tokio::sync::watch::Sender<bool>>,
    search_content: String, // uhm, needs state reference here idk. Option?
    tags_list: Option<Vec<Tag>>,
    selected_file_idx: Option<usize>,
    error_message: Option<String>,
    collection: Option<Collection>
}
impl Default for Counter {
    fn default() -> Self {
        let default_directory = default_dir();
        let file_list = file::image_list(&default_directory);
        let shutdown_signal = None;

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

        return Self {
            panel: Panel::None,
            directory_list: file::directory_list(&default_directory),
            file_list: file_list,
            file_filter_indices: None,
            directory: default_directory,
            shutdown_signal: shutdown_signal,
            search_content: String::from(""),
            tags_list: None,
            selected_file_idx: None,
            error_message: None,
            collection: None,
            
        }
    }
}

enum Panel {
    None, 
    File,
    Search,
    Tag,
    Collection, // bastardization of "Panel"
}

#[derive(Debug, Clone)]
pub enum Message {
    File,
    Search,
    Tag,
    SwitchDirectory(u16), // index
    ImageLoaded(usize, Handle), // index into image array, with its handle
    AllImagesLoaded,
    AddTag(TagIdx), // add an already existing tag to the selected file
    RemoveTag(TagIdx), // remove an already existing tag from the selected file
    AddInputTag, // reference search
    SearchChanged(String),
    ImageSelected(usize), // index into image array, indicating selection
    SearchSend,
    Save,
    AddPositiveSearchTerm(TagIdx),
    AddNegativeSearchTerm(TagIdx),
    Collection,
    ToggleIntoCollection(usize), // index into image array, indicating selection
    CollectionLeft,
    CollectionRight,
    CollectionImageAllocated(Allocation, usize), // index into image array
    CollectionImageFailed(usize), // index into image array

}

impl Counter {
    pub fn view<'a>(&'a self) -> Container<'a, Message> {

        // TODO we can maybe store this in state.
        let dir_name: String = match self.directory.as_os_str().to_str() {
            None => String::from("?"),
            Some(i) => i.to_owned()
        };

        let collection_name: String = match self.collection.as_ref() {
            None => String::from("Collection"),
            Some(i) => format!("Collection ({})", i.entries.len())
        };

        let top_bar: Container<Message> = container(row![
            button("File").on_press(Message::File),
            button("Search").on_press(Message::Search),
            button("Tag").on_press(Message::Tag),
            button(text(collection_name)).on_press(Message::Collection),
            text(dir_name).width(Fill).center(),
            button("Save").on_press(Message::Save)
        ].align_y(iced::Alignment::Center).spacing(5).padding(5)).width(Fill);
        let left_panel;
        
        
        {
            let left_panel_width: u32 = match self.panel {
                Panel::None | Panel::Collection => 0,
                _ => 300
            };

            let left_contents: Column<Message> = match self.panel {
                Panel::File => {
                    self.get_file_panel()
                },
                Panel::Search => {
                    self.get_search_panel()
                },
                Panel::Tag => {
                    self.get_tag_panel()
                },
                _ => column![]
            };

            left_panel = container(scrollable(left_contents)).style(container::bordered_box).width(left_panel_width).height(Fill);
        }

        let body: Element<Message> = match self.panel {
            Panel::Collection => self.get_collection_body(),
            _ => scrollable(self.get_image_grid()).width(Fill).height(Fill).into(),
        };
        let main_container = container(
            row![
                left_panel,
                body
            ].spacing(10)
        ).padding(10).style(container::bordered_box).width(Fill).height(Fill);

        let full_app = container(
            column![
                top_bar,
                self.get_error_message_panel(),
                main_container
            ].spacing(10)
        );

        full_app
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        // destroy error message
        self.error_message = None;
        match message {
            Message::File => {
                self.panel = match self.panel {
                    Panel::File => Panel::None,
                    _ => Panel::File
                };
                Task::none()
            },
            Message::Search => {
                self.panel = match self.panel {
                    Panel::Search => Panel::None,
                    _ => Panel::Search
                };
                Task::none()
            },
            Message::Tag => {
                self.panel = match self.panel {
                    Panel::Tag => Panel::None,
                    _ => Panel::Tag
                };
                Task::none()
            },
            Message::SwitchDirectory(idx) => {
                self.go_to_dir(Some(idx as usize))
            },
            Message::ImageLoaded(idx, handle) => {
                match self.file_list.as_mut() {
                    Some(list) => {
                        if idx < list.len() {
                            list[idx].thumbnail_handle = Some(handle);
                        }
                        Task::none()
                    },
                    None => Task::none(),
                }
            },
            Message::AllImagesLoaded => {
                self.shutdown_signal = None; // todo is this thread safe?
                Task::none()
            },
            Message::AddTag(tidx) => { // existing tag
                self.add_tag_to_selected(tidx);
                Task::none()
            },
            Message::RemoveTag(tidx) => { // existing tag

                // ensure file selected
                let file_idx = match self.selected_file_idx {
                    None => return Task::none(), // no file selected tf???
                    Some(v) => v
                };

                // now, get the img list
                let file_list_obj = match self.file_list.as_mut() {
                    None => return Task::none(),
                    Some(list) => list
                };

                // get the actual image
                let img = &mut file_list_obj[file_idx];
                match img.tags.as_mut() {
                    None => return Task::none(),
                    Some(tlist) => {
                        // need to check if the tag exists in here.
                        let idx = match tlist.iter().position(|x| *x == tidx) {
                            None => return Task::none(),
                            Some(id) => id
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
                    Some(list) => list
                };

                // get object in list
                let tag = &mut tag_list_obj[tidx as usize];

                if tag.refs == 0 {
                    self.error_message = Some(String::from("Warning! We removed a tag that had zero references. Something is off here."));
                    return Task::none();
                    // this might indicate a bug, or a user who spammed the minus button.
                }
                tag.refs -= 1;
    
    


                // so, we only add the tag and increment ref if both succeed.


                Task::none()
            },
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
                    },
                    Some(l) => l
                };

                // get index of tag if already exists, or none otherwise
                let idx_opt = tags_list.iter().position(|tag| tag.name == self.search_content);

                match idx_opt {
                    None => {
                        let next_idx = match tags_list.last() {
                            Some(i) => i.idx+1,
                            None => 0,
                        };
                        tags_list.push(Tag { name: self.search_content.clone(), idx: next_idx, refs: 0 }); // make with 0, AddTag will handle that
                        self.search_content = String::from("");
                        self.add_tag_to_selected(next_idx);
                    },
                    Some(i) => self.add_tag_to_selected(i as u32)
                }

                Task::none()
            },
            Message::SearchChanged(str) => {
                self.search_content = str;
                Task::none()
            },
            Message::ImageSelected(idx) => {
                match self.selected_file_idx {
                    Some(i) if i == idx => self.selected_file_idx = None,
                    _ => self.selected_file_idx = Some(idx)
                }
                Task::none()
            },
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

                let filter_indices = match search::filter_to_string(self.search_content.clone(), tags_list, images) {
                    Ok(v) => v,
                    Err(s) => {
                        //whatever
                        self.file_filter_indices = None;
                        self.error_message = Some(s);
                        return Task::none();
                    },
                };
                println!("Got these filter indices: {:?}", filter_indices);

                self.file_filter_indices = Some(filter_indices);

                // need to tell the image loader to load smth else now
                if let Some(i) = self.shutdown_signal.take() {
                    let _ = i.send(true); // don't care if fails.
                    // if it fails that's good, we want to shut down.
                }

                // calculate the ones that need to be loaded
                let imgs_to_load: Vec<(usize, PathBuf)> = match self.file_list.as_ref() {
                    Some(file_list) => {

                        self.file_filter_indices.as_ref().unwrap().iter().map(|x| (x,&file_list[*x])).filter(|x| x.1.thumbnail_handle == None).map(|x| (*x.0, x.1.path.clone())).collect()
                    },
                    None => return Task::none(),
                };

                // tell it to work on these kitties
                let (msg_stream, shutdown_tx) = loader::get_async_values(imgs_to_load, THUMBNAIL_RES);
                self.shutdown_signal = Some(shutdown_tx);
                Task::run(msg_stream, |x| x)
            }
            Message::Save => {
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
    
                self.error_message = None;
                Task::none()
            },
            Message::AddPositiveSearchTerm(tidx) => {
                if let Some(l) = self.tags_list.as_ref() {

                    // TODO actually, I hate all of this.
                    // I would prefer that the search bar does some
                    // sort of auto complete.
                    self.search_content += " ";
                    self.search_content += &l[tidx as usize].name;
                }
                return Task::none();
            },
            Message::AddNegativeSearchTerm(tidx) => {
                if let Some(l) = self.tags_list.as_ref() {
                    self.search_content += " -";
                    self.search_content += &l[tidx as usize].name;
        
                }
                return Task::none();
            },
            Message::Collection => {
                self.panel = Panel::Collection;
                Task::none()
            },
            Message::ToggleIntoCollection(idx) => {
                match self.collection.as_mut() {
                    None => {

                        // make new

                        let img: &Img = match self.file_list.as_ref() {
                            None => return Task::none(), // ????
                            Some(list) => &list[idx]
                        };

                        let entry_name = img.name.clone();

                        let first_elt = CollectionElement {
                            img_idx: idx,
                            name: entry_name, // need to load it
                            alloc: None,
                        };

                        self.collection = Some(Collection {
                            selected: 0,
                            entries: vec![first_elt]
                        });

                        let path_clone = img.path.clone();
                        let handle = Handle::from_path(path_clone);

                        allocate(handle).map(move |res| {
                            match res {
                                Ok(alloc) => Message::CollectionImageAllocated(alloc, idx),
                                Err(_) => Message::CollectionImageFailed(idx),
                            }
                        })
                    },
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
                            },
                            None => {
                                // add the entry

                                let img: &Img = match self.file_list.as_ref() {
                                    None => return Task::none(), // ????
                                    Some(list) => &list[idx]
                                };

                                let entry_name = img.name.clone();

                                let new_element = CollectionElement {
                                    img_idx: idx,
                                    name: entry_name, // need to load it
                                    alloc: None,
                                };

                                col.entries.push(new_element);

                                let path_clone = img.path.clone();
                                let handle = Handle::from_path(path_clone);

                                allocate(handle).map(move |res| {
                                    match res {
                                        Ok(alloc) => Message::CollectionImageAllocated(alloc, idx),
                                        Err(_) => Message::CollectionImageFailed(idx),
                                    }
                                })
                    
                            },
                        }
                    }
                }
            },
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
            },
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
            },
            Message::CollectionImageAllocated(allocation, idx) => {
                if let Some(col) = self.collection.as_mut() {
                    if let Some(entry) = col.entries.iter_mut().rev().find(|elt| elt.img_idx == idx) {
                        entry.alloc = Some(allocation); 
                    }
                    // ignore if missing
                }
                // ignore if missing
                Task::none()
            },
            Message::CollectionImageFailed(_) => {
                println!("Allocating the collection image failed. Not sure what to do!");
                Task::none()
            },
        }
    }

    fn add_tag_to_selected(&mut self,tidx: TagIdx) {
        // ensure file selected
        let file_idx = match self.selected_file_idx {
            None => {
                return;
            }, // no file selected tf???
            Some(v) => v
           };

        // ok, get the list
        let tag_list_obj = match self.tags_list.as_mut() {
            None => {
                return;
            }, // no tasks available...?
            Some(list) => list
        };

        // get object in list
        let tag = &mut tag_list_obj[tidx as usize];
        
        

        // now, get the img list
        let file_list_obj = match self.file_list.as_mut() {
            None => {
                return;
            },
            Some(list) => list
        };

        // get the actual image
        let img = &mut file_list_obj[file_idx];
        match img.tags.as_mut() {
            None => {
                let mut tag_vec_temp = Vec::new();
                tag_vec_temp.push(tidx);
                img.tags = Some(tag_vec_temp);
                tag.refs += 1;
            },
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
        let mut i=0;
        for elt in list {
            col = col.push(button(text(&elt.name).width(Fill)).on_press(Message::SwitchDirectory(i)));
            i+=1;
        }
        col.spacing(4).padding(4)
    }

    pub fn get_image_grid<'a>(&'a self) -> Grid<'a, Message> {
        // column of rows
        // first, get the images
        if let None = self.file_list {

            return Grid::new().push("Failed to get files in this directory.");
        }
        let list = self.file_list.as_ref().unwrap();
        if list.len() == 0 {
            return Grid::new().push("There's no images here.");
        }
        let filtered_img_list: Vec<&Img> = match self.file_filter_indices.as_ref() {
            Some(l) => l.iter().map(|x| &list[*x]).collect::<Vec<&Img>>(),
            None => list.iter().map(|x| x).collect::<Vec<&Img>>()
        };
        // need to filter

        let grid = filtered_img_list.iter().fold(Grid::new().spacing(10),|grid: Grid<Message>, entry| {
            let container_content: Element<'_, Message> = match entry.thumbnail_handle.as_ref() {
                Some(handle) => {
                    MouseArea::new(
                        Image::new(
                            handle.clone()
                        ).width(THUMBNAIL_RES).height(THUMBNAIL_RES).content_fit(iced::ContentFit::Cover)
                    ).on_press(Message::ImageSelected(entry.idx)).on_right_press(Message::ToggleIntoCollection(entry.idx)).into()
                },
                None => Text::new("Loading...").into(),
            };
            grid.push(
                container( 
                    column![
                        container(
                            container_content
                        ).height(Fill).width(Fill),
                        text(&entry.name).height(BOX_TEXT_HEIGHT).width(Fill),

                    ].width(Fill).height(Fill)
                ).width(Fill).style(container::bordered_box) // height thumbnail res?
            )
        } );

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
        let top_row = row![
            iced::widget::text_input("Enter tag...", &self.search_content).width(Fill).on_input(|s| Message::SearchChanged(s)),
            button("+").on_press(Message::AddInputTag)
        ].width(Fill);

        let mut col = Column::new().push(top_row);

        if let None = self.selected_file_idx {
            
            return col.push(text("Select a file to add tags."));
        }

        if let None = self.tags_list {
            return col.push(text("Add your first tag."));
        }

        let list = self.tags_list.as_ref().unwrap(); // presume it exists

        col = list.iter().fold(col, |col: Column<Message>, tag| {
            col.push(row![
                text(tag.name.clone()).width(Fill),
                button("+").on_press(Message::AddTag(tag.idx)),
                button("-").on_press(Message::RemoveTag(tag.idx))
            ])
        });

        col = col.push(self.get_tags_on_image_container(Some(|tidx| {
            vec![
                button("-").on_press(Message::RemoveTag(tidx))
            ]
        })));

        col
    }

    pub fn get_search_panel<'a>(&'a self) -> Column<'a, Message> {
        let top_row = row![
            iced::widget::text_input("Search...", &self.search_content).width(Fill).on_input(|s| Message::SearchChanged(s)),
            button("Go").on_press(Message::SearchSend)
        ].width(Fill);

        let mut col = Column::new().push(top_row);

        col = col.push(self.get_tags_on_image_container(Some(|tidx| {
            vec![
                button("+").on_press(Message::AddPositiveSearchTerm(tidx)),
                button("-").on_press(Message::AddNegativeSearchTerm(tidx)),
            ]
        })));

        col

    }

    fn get_error_message_panel<'a>(&'a self) -> Element<'a, Message> {
        match self.error_message.as_ref() {
            Some(value) => text(value).height(80).width(Fill).into(),
            None => text("").height(0).width(Fill).into()
        }
    }

    ///
    /// Gets the body for viewing a collection of images.
    /// A collection of images is something the user decides. They can add
    /// images to the collection, and also subtract these images. The collection
    /// view allows users to see the images at maximal resolution and cycle
    /// through with buttons (or later arrow keys)
    fn get_collection_body<'a>(&'a self) -> Element<'a, Message> {
        let (collection_entries, selected_idx) = match self.collection.as_ref() {
            Some(i) => (&i.entries, i.selected),
            None => return text("Make a collection first!").into()
        };

        let col_entry = &collection_entries[selected_idx];

        let main_elt: Element<'a, Message> = match col_entry.alloc.as_ref() {
            Some(alloc) => {
                Image::new(alloc.handle()).width(Fill).height(Fill).content_fit(iced::ContentFit::ScaleDown).into()
            },
            None => text("Loading...").center().into()
        };

        let center_stack = column![
            text(col_entry.name.clone()).height(80).width(Fill).center(),
            main_elt
        ];
        let entire_panel = row![
            button("<").on_press(Message::CollectionLeft),
            center_stack.height(Fill).width(Fill),
            button(">").on_press(Message::CollectionRight)
        ].width(Fill).height(Fill).align_y(Vertical::Center);

        container(entire_panel).width(Fill).height(Fill).style(container::bordered_box).into()
    }
    ///
    /// Look. I know it's a hellish type.
    /// You have the OPTION to generate buttons per tag.
    /// You can have as many buttons that you want, which will be put in a row format.
    fn get_tags_on_image_container<'a>(&'a self, button_generator: Option<fn(TagIdx) -> Vec<Button<'a, Message>>>) -> Element<'a, Message> {

        let mut col = Column::new();
        col = col.push(text("Tags on this image"));

        let contain = |c: Column<'a, Message>| -> Element<'a, Message> {
            container(c.width(Fill).spacing(3)).padding(5).into()
        };

        let contain_empty = |c: Column<'a, Message>| -> Element<'a, Message> {
            contain(c.push(text("None")))
        };

        if let None = self.selected_file_idx {
            return contain_empty(col);
        }

        if let None = self.file_list {
            return contain_empty(col); // shouldn't be possible given above is None, but good to check i guess.
        }

        if let None = self.tags_list {
            return contain_empty(col);
        }

        let selected_file: &Img = &self.file_list.as_ref().unwrap()[*self.selected_file_idx.as_ref().unwrap()];

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
            Some(l) => l
        };
        match idx {
            None => file::default_dir(),
            Some(i) => {
                list[i].path.clone()
            }
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
    fn go_to_dir(&mut self, idx: Option<usize>) -> Task<Message> {
        // first, try to save and compress.
        self.compress_and_save_to_file();

        

        self.tags_list = None;
        self.file_filter_indices = None;
        self.collection = None;
        
        self.directory = self.get_dir_from(idx);
        self.directory_list = file::directory_list(&self.directory);

        self.selected_file_idx = None;
    
        if let Some(i) = self.shutdown_signal.take() {
            let _ = i.send(true); // don't care if fails.
            // if it fails that's good cuz we want it to shut down.
        }
        // self.shutdown_signal guaranteed to be None now.
        // tokio thread will stop when it sees.

        // 


        match file::image_list(&self.directory) {
            None => {
                self.file_list = None;
                Task::none()
            }
            Some(list) => {
                let simple_list = list.iter().enumerate().map(|entry| (entry.0, entry.1.path.clone())).collect();
                self.file_list = Some(list);
                // read metadata
                match file::try_get_metadata_path(&self.directory) {
                    Some(metadata_path) => self.tags_list = file::read_metadata(&metadata_path, self.file_list.as_mut().unwrap()),
                    None => {}
                }
                let (msg_stream, shutdown_tx) = loader::get_async_values(simple_list, THUMBNAIL_RES);
                self.shutdown_signal = Some(shutdown_tx);
                Task::run(msg_stream, |x| x)
            },
        }
    }


}

pub fn is_valid_tag(name: &str) -> bool {
    if name.len() == 0 || !name.is_ascii() {
        return false;
    }
    for c in name.chars() {
        match c {
            '0'..='9' | 'a'..='z' | 'A'..='Z' | '_' => {},
            _ => return false
        }
    }
    return true;
}

#[tokio::main]
async fn main() {
    let _ = iced::run(Counter::update, Counter::view);
}

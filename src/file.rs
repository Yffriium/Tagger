use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use iced::widget::image::Handle;
use std::fs;

const IMG_FILE_EXTS: [&'static str; 2] = ["png", "jpg"]; // put valid file extensions here
pub const METADATA_NAME: &str = "tag.yff";

pub type TagIdx = u32;
#[derive(Debug)]
pub struct Tag {
    pub name: String,
    pub idx: TagIdx,
    pub refs: u32 // how many files reference this tag
}

impl Tag{
    fn as_bytes(&self) -> Vec<u8> {
        let mut vec: Vec<u8> = Vec::new();
        vec.extend(self.idx.to_le_bytes());
        vec.extend(self.refs.to_le_bytes());
        vec.extend(self.name.as_bytes());
        vec
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        let idx: TagIdx = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let refs: u32 = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let name: String = std::str::from_utf8(&bytes[8..]).expect("Hey! Failed to get Tag string. crashing!").to_owned();

        Tag { name, idx, refs }
    }
}

pub struct Dir {
    pub path: PathBuf,
    pub name: String
}

#[derive(Debug)]
pub struct Img {
    // put information relevant to images in here
    pub idx: usize, // index into the array this is in
    pub path: PathBuf,
    pub name: String,
    pub extension: String,
    pub thumbnail_handle: Option<Handle>,
    pub tags: Option<Vec<TagIdx>>
}

pub fn default_dir() -> PathBuf {
    return env::current_dir().expect("Hey! We don't have perms to see this directory.");
}

pub fn image_list(curr: &PathBuf) -> Option<Vec<Img>> {
    if !curr.is_dir() {
        return None;
    }
    let mut list: Vec<Img> = Vec::new();

    let files = match curr.read_dir() {
        Ok(v) => v,
        Err(_) => return None
    };

    for entry in files {
        match entry {
            Ok(v) => {
                let p = v.path();
                if p.is_file() {
                    let ext_name: String = match p.extension() {
                        Some(z) => {
                            match z.to_str() {
                                Some(r) => r.to_owned(),
                                None => continue,
                            }
                        },
                        None => continue
                    };

                    // TODO:
                    // Can swap to binary search if extensions are sorted.
                    // TODO
                    // wait, i couldn't get .contains to even work .Hmm.
                    let mut found = false;
                    for other_str in IMG_FILE_EXTS {
                        if other_str == ext_name {
                            found = true;
                            break;
                        }
                    }
                    if !found {
                        continue
                    }
                    
                    // good! add to list.
                    let os_file_name: OsString = v.file_name();
                    let name: String = match os_file_name.to_str() {
                        None => continue,
                        Some(x) => x.to_owned()
                    };
                    list.push(Img { idx: list.len(), path: p, name: name, extension: ext_name, thumbnail_handle: None, tags: None });
                }
            },
            Err(_) => continue, // the hell?
        }
    }

    Some(list)
}

pub fn directory_list(curr: &PathBuf) -> Option<Vec<Dir>> {
    if !curr.is_dir() {
        return None;
    }
    let mut list: Vec<Dir> = Vec::new();

    let files = match curr.read_dir() {
        Ok(v) => v,
        Err(_) => return None
    };
    let parent_path: PathBuf = match curr.parent() {
        Some(v) => v.to_path_buf(),
        None => return None
    };

    list.push(Dir { path: parent_path, name: String::from("..") });

    for entry in files {
        match entry {
            Ok(v) => {
                let p = v.path();
                if p.is_dir() {
                    // good! add to list.
                    let os_file_name: OsString = v.file_name();
                    let name: &str = match os_file_name.to_str() {
                        None => continue,
                        Some(x) => x
                    };
                    list.push(Dir { path: p, name: name.to_owned() })
                }
            },
            Err(_) => continue, // the hell?
        }
    }

    Some(list)
}

/// None if no metadata here. PathBuf of metadata otherwise.
pub fn try_get_metadata_path(directory: &PathBuf) -> Option<PathBuf> {
    let files = match directory.read_dir() {
        Err(_) => {
            println!("Could not get path to metadata! Error on reading dir.");
            return None
        },
        Ok(i) => i
    };

    for entry in files {
        match entry {
            Ok(dirent) => {
                let path = dirent.path();
                if path.is_file() {
                    if let Some(i) = dirent.file_name().to_str() {
                        if i == METADATA_NAME {
                            return Some(path);
                        }
                    }
                }
            },
            _ => continue,
        }
    }
    None
}
///
/// Lots of safety problems here. Could crash program if file is made wrong,
/// or if file was edited by user.
/// TODO go in and fix the bounds checks and safeties. 
/// 
/// Modifies the image vector itself
pub fn read_metadata(metadata_path: &PathBuf,images: &mut Vec<Img>) -> Option<Vec<Tag>> {
    let contents = match fs::read(metadata_path) {
        Ok(v) => v,
        _ => return None,
    };

    // presume metadata is all ascii
    // FORMAT: 
    // First line is the tag array.
    // Tags start with id, references, and then their name.
    // Tags are ended with the 0 byte (even the last tag!)
    // No more tags to list once we reach \n.
    // Each other line represents a file.
    // The first part is the file's full name in the folder.
    // Then, there is the \0 separator.
    // Then, there are a list of tag numbers (as u32) separated by nothing.

    let mut tags: Vec<Tag> = Vec::new();

    let mut seek_idx = 0;
    'tag_loop: loop {
        if contents[seek_idx] == b'\n' {
            // we're done
            break;
        }

        // now for name
        let tag_start = seek_idx;
        seek_idx += 8; // first 8 are numbers, MUST skip
        'string_loop: loop {
            seek_idx += 1; // increment first bc tag has non-zero length
            match contents[seek_idx] {
                0 => {
                    let found_tag = Tag::from_bytes(&contents[tag_start..seek_idx]);
                    tags.push(found_tag);
                    seek_idx += 1; // keep going
                    break 'string_loop;
                },
                _ => {}

            }
        }
    }
    // we now have our tag array
    seek_idx += 1;
    // we are now seeked at the start of the first file

    // for each line...
    'file_loop: loop {
        if seek_idx >= contents.len() {
            break; // we are done here
        }

        // first, get the file name
        let name_start = seek_idx;
        let file_name: String = 'name_loop: loop {
            seek_idx += 1;
            match contents[seek_idx] {
                0 => {
                    let found_tag = match std::str::from_utf8(&contents[name_start..seek_idx]) {
                        Ok(t) => t,
                        Err(_) => return None,
                    };
                    break 'name_loop found_tag.to_owned();
                },
                _ => {}
            }
        };
        // we now have the file name owned
        // we are at the \0 char right after the file name

        seek_idx += 1; // pointing to first tag
        let mut tidx_vec: Vec<TagIdx> = Vec::new();
        'tag_loop: loop {
            if contents[seek_idx] == b'\n' || seek_idx >= contents.len() {
                break 'tag_loop;
            }
            let tidx: TagIdx = u32::from_le_bytes(contents[seek_idx..seek_idx + 4].try_into().unwrap()); // TODO so unsafe...
            tidx_vec.push(tidx);
            seek_idx += 4;
        }
        seek_idx += 1; // now seeked at start of next line, ready

        // we now have the tag indices for the image file
        // try to find the file by name
        let img = match find_img_idx_by_name(&file_name, images) {
            Some(i) => i,
            None => {
                // what?
                // didn't find?
                println!("Hey! We didn't find an image from the file.");
                continue; // just go next
            },
        };

        img.tags = match tidx_vec.len() {
            0 => None,
            _ => Some(tidx_vec)
        };
    }

    println!("Found tags. Got these: {:?}", tags);
    
    Some(tags)
}

///
/// Right now this is inefficient.
/// TODO: Enforce ordering on images array by name, then can binary search.
pub fn find_img_idx_by_name<'a>(compare: &str, images: &'a mut [Img]) -> Option<&'a mut Img> {
    images.iter_mut().find(|img| img.name == compare)
}

pub fn write_metadata(metadata_path: &PathBuf, tags: &Vec<Tag>, images: &Vec<Img>) -> bool {
    // presume metadata is all ascii
    // FORMAT: 
    // First line is the tag array.
    // Tags separated by \0 character.
    // No more tags to list once we reach \n.
    // Each other line represents a file.
    // The first part is the file's full name in the folder.
    // Then, there is the \0 separator.
    // Then, there are a list of tag numbers (as u32) separated by nothing.

    let mut contents: Vec<u8> = Vec::new();
    for tag in tags {
        contents.extend(tag.as_bytes());
        contents.push(0);
    }
    contents.push(b'\n');

    for img in images {
        contents.extend(img.name.as_bytes());
        contents.push(0);
        if let Some(list) = img.tags.as_ref() {
            for tidx in list {
                contents.extend(tidx.to_le_bytes());
            }
        }
        contents.push(b'\n');
    }

    match fs::write(metadata_path, contents) {
        Ok(_) => true,
        Err(_) => false,
    }
}

///
/// From a tag list (which may have empty slots), identifies a mapping from old tag indices
/// to new tag indices.
/// We can then go through the files and perform this shift.
/// 
fn new_mappings(tags: &Vec<Tag>) -> Vec<Option<TagIdx>> {
    let mut counter = 0;
    let mut vec: Vec<Option<TagIdx>> = Vec::new();
    for entry in tags {

        match entry.refs {
            0 => {
                // add an empty entry, indicating that the TagIdx here maps to nothing
                vec.push(None);
            },
            _ => {
                vec.push(Some(counter));
                counter += 1;
            }   
        }
    }
    vec
}

pub fn compress_tags(tags: &mut Vec<Tag>, images: &mut Vec<Img>) {
    let new_map = new_mappings(tags);

    tags.retain(|x| x.refs > 0);

    for tag in tags {
        tag.idx = new_map[tag.idx as usize].unwrap(); // unwrap shouldn't panic
    }

    for image in images {
        match image.tags.as_mut() {
            Some(tag_list) => {
                let new_tags_list: Vec<u32> = tag_list.iter().map(|x| new_map[*x as usize].unwrap()).collect();
                image.tags = Some(new_tags_list);
            },
            None => {},
        }
    }
}
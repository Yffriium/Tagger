# Tagger
Tagger is a program made entirely in Rust using iced GUI. It lets users add tags to local images on their computer. Then, the user can filter images based on these tags. Images can also be quickly added to and removed from the "Collection", which is a temporary group of images that can be cycled through easily. Tagger leaves a single lightweight file within any directories with tagged images. This file stores the tag information for each image.

For instance, a user could tag images from a vacation with the names of everyone who appears in each image. Then, they can filter to just images which contain both 'Casey' and 'Riley' tags. Lastly, they can look through and select some of these images to add to the Collection, which will let them view just these images at a high resolution.

## Installation
If you use the same architecture as my computer (`x86_64-pc-windows-msvc`), then simply download the included `tagger.exe` file and run this. Otherwise, you will need `cargo` to build this project for your own computer.

Download this project to your computer.

In the terminal, navigate to the directory for this project.

Type `cargo build --release`. This will build the program for your hardware in the `target/release` folder. For windows, it is `target/release/tagger.exe`.

Find the runnable application, and run!

## How to Use
There are three panel selection buttons in the top left.

### File
Allows you to navigate through directories to find which directory you'd like to tag images within. The top option moves up to the parent directory. All other options show child directories. The current directory is shown at the top of the screen.
### Search
Type in any number of tags, then press 'Go' to search for images containing all of these tags. Add a `-` before a tag to exclude it. Empty the search and press 'Go' to remove all filters.
### Tag
Adds tags to the currently selected image. Select an image using the mouse or the arrow keys. Type a tag in at the top and press '+' to assign it to the image. Alternatively, use the '+' and '-' options on existing tags to add/remove them from the image.

### Help
In the top right, there is a Help toggle. Click this to toggle Help on/off, which will put help information at the bottom of the screen for the most recently touched function.

The main region has two different panels.


### Explore
Use this to see all the images within the directory, to see search results, and to select images. Images can also be added to the collection in this panel. Use left mouse or arrow keys to select images. Scroll down to see more images, if there are too many. Right click or press space to add/remove images from the Collection.

### Collection
Use this to see all the images you have explicitly chosen to add for this session. The Collection is most useful for viewing a particular subset of images in high resolution. Use space to remove images from the collection, and arrow keys to navigate between collection contents. In the Collection menu, you can also reveal the file within the system file explorer by pressing the associated button.
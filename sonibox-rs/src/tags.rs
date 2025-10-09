//! Set up all the RFID tags for the application that we need to check for.

/// These are all the tags that are set up.
///
/// Unique tags, in order of folder, must come first. For all additinal tags at the end of the
/// list, the folder selection will wrap around.
static TAGS: [[u8; 4]; 11] = [
    [0x2E, 0x52, 0xE8, 0xC1], // Tag for Folder 1
    [0xCE, 0x46, 0xE8, 0xC1], // Tag for Folder 2
    [0x3E, 0x43, 0xE8, 0xC1], // Tag for Folder 3
    [0xBE, 0x56, 0xE8, 0xC1], // Tag for Folder 4
    [0x0E, 0x54, 0xE8, 0xC1], // Tag for Folder 5
    [0xDE, 0x86, 0xE7, 0xC1], // Tag for Folder 6
    [0x5E, 0x56, 0xE8, 0xC1], // Tag for Folder 7
    [0xCE, 0x54, 0xE8, 0xC1], // Tag for Folder 8
    [0x1E, 0x16, 0xE8, 0xC1], // Tag for Folder 9
    [0x06, 0x99, 0xC1, 0x44], // Development card F1
    [0x95, 0xE0, 0x7D, 0x6C], // Development card F2
];

/// This is the number of unique tags that are used.
static NUMBER_UNIQUE_TAGS: usize = 9;

/// If a known tag is found, return its index, otherwise None
pub fn folder_selection(uid: &[u8]) -> Option<usize> {
    for (ind, val) in TAGS.iter().enumerate() {
        if *val == uid {
            return Some(ind % NUMBER_UNIQUE_TAGS + 1);
        }
    }
    None
}

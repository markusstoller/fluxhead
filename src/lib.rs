extern crate core;

use endian_codec::{DecodeLE, EncodeLE, PackedSize};

#[derive(Clone, Debug, PartialEq, Eq, PackedSize, EncodeLE, DecodeLE)]
struct CbmHeaderInfo {
    pub header_block_id: u8,
    header_block_checksum: u8,
    pub sector: u8,
    pub track: u8,
    format_id_low: u8,
    format_id_high: u8,
    reserved: u8,
    reserved2: u8,
}

//use statemachine::StateMachine;
#[derive(Clone, Debug, PartialEq, Eq, PackedSize, EncodeLE, DecodeLE)]
pub struct G64HeaderInfo {
    tag: [u8; 8],
    revision: u8,
    total_tracks: u8,
    track_size: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, PackedSize, EncodeLE, DecodeLE)]
struct TrackLocation {
    track_offset: u32,
    half_track_offset: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, PackedSize, EncodeLE, DecodeLE)]
struct G64TrackData {
    track_data_size: u16,
}

const TRACK_HEADER_SIZE: usize = 10;
const TRACK_DATA_SIZE: usize = 325;
const PATTERN_SIZE: usize = 4;
const TRACK_HEADER_IDENTIFIER: u8 = 0x52;
const TRACK_DATA_IDENTIFIER: u8 = 0x55;

#[derive(Clone)]
pub struct SectorHeader {
    pub gcr_encoded_data: Vec<u8>,
    pub gcr_decoded_data: Vec<u8>,
}

pub struct SectorData {
    pub gcr_encoded_data: Vec<u8>,
    pub gcr_decoded_data: Vec<u8>,
}

pub struct Sector {
    pub header: SectorHeader,
    pub data: SectorData,
}

pub struct Track {
    pub tracks: Option<Vec<Sector>>,
    pub speed_zone: i32,
}

pub struct ParsedData {
    pub g64_header: Option<G64HeaderInfo>,
    pub tracks: Option<Vec<Track>>,
}

struct Fluxhead {
    data: Vec<u8>,
}

impl Fluxhead {
    /// Creates a new instance of the struct with an empty `data` vector.
    ///
    /// # Returns
    ///
    /// * A new instance of the struct, initialized with an empty `data` field.
    ///
    /// # Example
    ///
    /// ```
    /// let instance = StructName::new();
    /// assert!(instance.data.is_empty());
    /// ```
    pub fn new() -> Self {
        Self { data: vec![] }
    }

    /// Loads the contents of a file at the given path into the `data` field.
    ///
    /// # Parameters
    /// - `path` (`String`): The path to the file to be loaded.
    ///
    /// # Behavior
    /// This function reads the contents of the file located at the specified `path`
    /// and assigns it to the `data` field of the struct.
    ///
    /// # Panics
    /// This function will panic if:
    /// - The file does not exist.
    /// - The application does not have permission to read the file.
    /// - An I/O error occurs while reading the file.
    ///
    /// # Example
    /// ```rust
    /// let mut instance = MyStruct { data: Vec::new() };
    /// instance.load_file(String::from("example.txt"));
    /// ```
    fn load_file(&mut self, path: String) -> Result<(), std::io::Error> {
        let res = std::fs::read(path)?;
        self.data = res;
        Ok(())
    }

    /// Finds the locations of specific synchronization marks in the given byte slice.
    ///
    /// This function scans through the input data and looks for consecutive patterns of
    /// `PATTERN_SIZE` bytes where all bytes are `0xFF`, and the next byte (after the pattern)
    /// has its most significant bit (MSB) not set (i.e., `& 0x80 == 0`). It then calculates
    /// the offsets of these synchronization marks within the data and returns them.
    ///
    /// # Parameters
    /// - `data`: A reference to a slice of bytes within which to search for synchronization marks.
    ///
    /// # Returns
    /// - `Option<Vec<usize>>`:
    ///   - `Some(Vec<usize>)` if one or more synchronization marks are found, where each `usize`
    ///     represents the offset in the input slice where a synchronization mark is located.
    ///   - `None` if no synchronization marks are found.
    ///
    /// # Notes
    /// - `PATTERN_SIZE` must be defined as a constant elsewhere in the code to specify the size
    ///   of the pattern to search for.
    /// - The function uses a sliding window technique to efficiently scan through the input data.
    /// - A synchronization mark consists of `PATTERN_SIZE` consecutive bytes with a value of `0xFF`,
    ///   followed by a byte where the MSB is not set.
    ///
    /// # Example
    /// ```
    /// const PATTERN_SIZE: usize = 4;
    /// let data: &[u8] = &[0xFF, 0xFF, 0xFF, 0xFF, 0x00, 0x12, 0x34, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F];
    /// let offsets = my_struct.find_sync_mark_locations(data);
    /// assert_eq!(offsets, Some(vec![4, 11]));
    /// ```
    fn find_sync_mark_locations(&self, data: &[u8]) -> Option<Vec<usize>> {
        let mut offsets: Vec<usize> = vec![];
        let mut ptr = 0;

        for window in data.windows(PATTERN_SIZE + 1) {
            if window[0..PATTERN_SIZE].iter().all(|&b| b == 0xFF) {
                if window[PATTERN_SIZE] & 0x80 == 0 {
                    let offset = ptr + PATTERN_SIZE;
                    offsets.push(offset);
                }
            }

            ptr += 1;
        }

        if offsets.len() > 0 {
            return Some(offsets);
        }

        None
    }

    /// Parses a given CBM DOS track to extract sectors and their data.
    ///
    /// # Arguments
    ///
    /// * `track` - A byte slice representing the raw track data to be parsed.
    ///
    /// # Returns
    ///
    /// Returns an `Option<Vec<Sector>>`:
    /// - `Some(Vec<Sector>)` if sectors were successfully found and parsed from the track.
    /// - `None` if no sectors could be found or parsed.
    ///
    /// # Behavior
    ///
    /// 1. Initializes an empty vector `found_tracks` to store discovered sectors.
    /// 2. Creates a new GCR decoder to handle decoding operations.
    /// 3. Prepares an empty `SectorHeader` to store header data during the parsing process.
    /// 4. Invokes the `find_sync_mark_locations` function to determine the positions of
    ///    possible sync marks in the track data.
    /// 5. For each sync mark position:
    ///     - Checks if it matches a track header identifier (`TRACK_HEADER_IDENTIFIER`):
    ///         - Processes the header using the `process_track_header` function,
    ///           updating the `sector_header`.
    ///     - Checks if it matches a track data identifier (`TRACK_DATA_IDENTIFIER`):
    ///         - Extracts the sector data using the `process_track_data` function.
    ///         - Combines the processed header and data into a `Sector`, adding it to `found_tracks`.
    /// 6. If `found_tracks` contains one or more sectors, returns it as `Some(Vec<Sector>)`.
    ///    If not, returns `None`.
    ///
    /// # Dependencies
    ///
    /// * The function relies on the `cbm_dos::GCR` decoding implementation for handling
    ///   GCR data.
    /// * The constants `TRACK_HEADER_IDENTIFIER` and `TRACK_DATA_IDENTIFIER` determine
    ///   how the track is parsed.
    /// * The helper functions `find_sync_mark_locations`, `process_track_header`, and
    ///   `process_track_data` are used to analyze and decode the track.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let track_data: &[u8] = /* raw track data */;
    /// let parser = TrackParser::new();
    /// match parser.parse_track(track_data) {
    ///     Some(sectors) => {
    ///         for sector in sectors {
    ///             println!("Sector Header: {:?}", sector.header);
    ///             println!("Sector Data: {:?}", sector.data);
    ///         }
    ///     },
    ///     None => {
    ///         println!("No sectors found in the track.");
    ///     }
    /// }
    /// ```
    ///
    /// # Notes
    ///
    /// * The track must be in a supported CBM DOS format, with identifiable sync marks,
    ///   track headers, and track data.
    /// * This function assumes that GCR encoding is used and delegates decoding to the
    ///   `cbm_dos::GCR` module.
    ///
    /// # Errors
    ///
    /// * This function does not explicitly handle errors; instead, it returns `None` if
    ///   no valid sectors are found during parsing.
    fn parse_track(&self, track: &[u8]) -> Option<Vec<Sector>> {
        let mut found_tracks: Vec<Sector> = vec![];
        let mut gcr_decoder = cbm_dos::GCR::new();
        let mut sector_header = SectorHeader {
            gcr_encoded_data: vec![],
            gcr_decoded_data: vec![],
        };

        if let Some(sync_offsets) = self.find_sync_mark_locations(track) {
            for offset in sync_offsets {
                if track[offset] == TRACK_HEADER_IDENTIFIER {
                    sector_header = self.process_track_header(track, offset, &mut gcr_decoder);
                }

                if track[offset] == TRACK_DATA_IDENTIFIER {
                    let sector_data = self.process_track_data(track, offset, &mut gcr_decoder);
                    found_tracks.push(Sector {
                        header: sector_header.clone(),
                        data: sector_data,
                    });
                }
            }
        }

        if !found_tracks.is_empty() {
            Some(found_tracks)
        } else {
            None
        }
    }

    /// Processes the track header by decoding the provided GCR-encoded data and extracting header information.
    ///
    /// # Arguments
    ///
    /// * `track` - A slice of bytes representing the track data to be processed.
    /// * `offset` - The starting position within the track slice from where the header processing begins.
    /// * `gcr_decoder` - A mutable reference to the GCR decoder used to decode the GCR-encoded data.
    ///
    /// # Returns
    ///
    /// Returns a `SectorHeader` struct containing:
    /// - `gcr_encoded_data`: The GCR-encoded header data extracted from the provided track slice.
    /// - `gcr_decoded_data`: The decoded header data obtained via the GCR decoder.
    ///
    /// # Errors
    ///
    /// This function assumes that the `gcr_decoder.decode` method never fails (via `unwrap`) and will panic if decoding the encoded data results in an error.
    ///
    /// # Example
    ///
    /// ```
    /// // Example usage of `process_track_header`
    /// let track_data: &[u8] = ...; // Byte slice representing track data
    /// let offset: usize = ...; // Starting position within the track data
    /// let mut gcr_decoder = cbm_dos::GCR::new(); // Initialize a GCR decoder
    ///
    /// let header = process_track_header(&track_data, offset, &mut gcr_decoder);
    ///
    /// println!("Encoded Data: {:?}", header.gcr_encoded_data);
    /// println!("Decoded Data: {:?}", header.gcr_decoded_data);
    /// ```
    ///
    /// # Notes
    ///
    /// - This function decodes the Commodore 1541 disk header using the GCR decoding method.
    /// - The header information (e.g., track and sector) can be extracted using the `CbmHeaderInfo::decode_from_le_bytes` method,
    ///   though this is currently commented out in the provided code.
    ///
    /// # Dependencies
    ///
    /// - Requires the `cbm_dos` module for the GCR decoding functionality.
    /// - Uses the `CbmHeaderInfo` struct for extracting information from the decoded header bytes.
    fn process_track_header(
        &self,
        track: &[u8],
        offset: usize,
        gcr_decoder: &mut cbm_dos::GCR,
    ) -> SectorHeader {
        let encoded_data = track[offset..offset + TRACK_HEADER_SIZE].to_vec();
        let decoded_data = gcr_decoder.decode(&encoded_data).unwrap();

        let hdr_info = CbmHeaderInfo::decode_from_le_bytes(&decoded_data);
        /*
                println!(
                    "Found Track Header: #{}:{}",
                    hdr_info.track, hdr_info.sector
                );
        */
        SectorHeader {
            gcr_encoded_data: encoded_data,
            gcr_decoded_data: decoded_data,
        }
    }

    /// Processes track data by extracting, decoding, and wrapping it into a `SectorData` structure.
    ///
    /// # Parameters
    /// - `track`: A slice of raw track data in byte form that needs processing.
    /// - `offset`: The start position within the `track` slice to begin extracting data.
    /// - `gcr_decoder`: A mutable reference to a `GCR` decoder for handling raw GCR-encoded data.
    ///
    /// # Returns
    /// A `SectorData` structure containing:
    /// - `gcr_encoded_data`: The raw track data extracted from the specified offset and encoded
    /// in GCR format.
    /// - `gcr_decoded_data`: The decoded data generated by the GCR decoder. If decoding fails, this
    /// will be an empty vector.
    ///
    /// # Behavior
    /// - A fixed-size portion of the `track` slice starting from the given `offset` is extracted.
    /// - The extracted data is passed to the provided GCR decoder for decoding.
    /// - If the decoding succeeds, the decoded data is included in the returned `SectorData`.
    /// - If the decoding fails, the `gcr_decoded_data` field of the `SectorData` is populated
    /// with an empty vector instead.
    ///
    /// # Notes
    /// - The code includes commented-out debug statements for printing out diagnostic data like
    /// the first 10 bytes of decoded data or failure messages.
    /// - `TRACK_DATA_SIZE` is assumed to be a constant defined elsewhere, representing the size
    /// of the portion of track data extracted for processing.
    ///
    /// # Example
    /// ```ignore
    /// let track_data: Vec<u8> = ...;
    /// let offset = 128;
    /// let mut gcr_decoder = cbm_dos::GCR::new();
    ///
    /// let sector_data = self.process_track_data(&track_data, offset, &mut gcr_decoder);
    ///
    /// if !sector_data.gcr_decoded_data.is_empty() {
    ///     println!("Successfully decoded data!");
    /// } else {
    ///     println!("Failed to decode GCR data.");
    /// }
    /// ```
    fn process_track_data(
        &self,
        track: &[u8],
        offset: usize,
        gcr_decoder: &mut cbm_dos::GCR,
    ) -> SectorData {
        let encoded_data = track[offset..offset + TRACK_DATA_SIZE].to_vec();

        if let Some(decoded_data) = gcr_decoder.decode(&encoded_data) {
            /*
                       println!(
                           "Found Track Data (first 10 bytes) {:x?}",
                           &decoded_data[0..10]
                       );

            */
            SectorData {
                gcr_encoded_data: encoded_data,
                gcr_decoded_data: decoded_data,
            }
        } else {
            /*
            println!("Failed to decode track data");
             */
            SectorData {
                gcr_encoded_data: encoded_data.clone(),
                gcr_decoded_data: vec![],
            }
        }
    }

    /// Processes and parses track data from the current object.
    ///
    /// This function extracts and decodes track data located at a specific offset in the `self.data`
    /// field. It performs the following operations:
    ///
    /// 1. Validates the provided `offset` to ensure it falls within the bounds of the available data.
    ///    - If the `offset` is invalid (0 or greater than the length of `self.data`), this function
    ///      returns `None`.
    /// 2. Decodes `G64TrackData` from the slice starting at the `offset`. This stores metadata about
    ///    the respective track.
    /// 3. Determines the start position of the actual track data by adding the size of
    ///    `G64TrackData` to the given `offset`.
    /// 4. Extracts the `capture` slice containing the track's raw data.
    /// 5. Attempts to parse the `capture` data into higher-level track sectors using the
    ///    `parse_track` method.
    ///    - If parsing is successful, a `Track` instance is created with the resulting sectors
    ///      and a default `speed_zone` value of 0, then returned.
    ///    - If parsing fails, `None` is returned.
    ///
    /// # Arguments
    ///
    /// * `offset` - The position within `self.data` where track information begins. Must
    ///   be greater than 0 and less than or equal to the length of the data.
    ///
    /// # Returns
    ///
    /// This function returns an `Option<Track>`, which can be:
    /// - `Some(Track)` if the track data was successfully decoded and parsed.
    /// - `None` if the `offset` is invalid or the track could not be parsed.
    ///
    /// # Panics
    ///
    /// This function does not explicitly handle panics, but may panic if the
    /// `offset` and data lengths are misaligned, causing slicing errors.
    ///
    /// # Examples
    ///
    /// ```rust
    /// let data = vec![...]; // G64 binary data
    /// let processor = YourProcessor { data };
    /// if let Some(track) = processor.process_track(10) {
    ///     println!("Track processed successfully: {:?}", track);
    /// } else {
    ///     println!("Failed to process track at offset 10.");
    /// }
    /// ```
    fn process_track(&self, offset: usize) -> Option<Track> {
        if offset == 0 || offset > self.data.len() {
            return None;
        }

        let track_data = G64TrackData::decode_from_le_bytes(&self.data[offset..]);
        let data_start = offset + size_of::<G64TrackData>();
        let capture = &self.data[data_start..data_start + track_data.track_data_size as usize];
        if let Some(sectors) = self.parse_track(capture) {
            return Some(Track {
                tracks: Option::from(sectors),
                speed_zone: 0,
            });
        }
        None
    }

    /// Parses the raw data stored in the struct into a structured `ParsedData` object.
    ///
    /// The method reads and decodes a `G64HeaderInfo` structure from the raw data
    /// and ensures its validity by asserting that the header tag matches the expected
    /// value (`[71, 67, 82, 45, 49, 53, 52, 49]` which corresponds to "GCR-1541").
    ///
    /// It then iterates over each track specified in the header, extracting the offset
    /// information for the full track and half track data along with their respective
    /// speed zone information. Using these offsets, the method processes each track
    /// and creates a list of parsed track data, associated with their respective
    /// speed zones. The resulting collection of track data and the header information
    /// are stored in the returned `ParsedData` object.
    ///
    /// # Returns
    /// - `Some(ParsedData)` if at least one valid header or track is parsed successfully.
    /// - `None` if no valid header or tracks could be processed.
    ///
    /// # Errors/Panics
    /// - Panics if the header tag does not match the expected value.
    ///   This ensures that the raw data being processed adheres to the expected format.
    ///
    /// # Expected Behavior
    /// - Validates and processes the G64 header and track data stored in `self.data`.
    /// - Accumulates the parsed tracks and associates appropriate speed zone information
    ///   for each full and half track.
    ///
    /// # Example Usage
    /// ```rust
    /// let parsed_result = instance.parse();
    /// if let Some(parsed_data) = parsed_result {
    ///     // Use parsed_data
    /// } else {
    ///     // Handle error case where parsing failed
    /// }
    /// ```
    ///
    /// # Internal Details
    /// - The method uses `G64HeaderInfo::decode_from_le_bytes` to decode the header
    ///   and `TrackLocation::decode_from_le_bytes` to decode track and speed zone offsets.
    /// - Offsets are determined based on the layout of the G64 data structure where
    ///   the header is followed by offsets for track locations and speed zones.
    /// - `process_track` is used to decode and process individual tracks using their offsets.
    ///
    /// # Dependencies
    /// The method relies on the following imported/internal types/functions:
    /// - `G64HeaderInfo::decode_from_le_bytes`
    /// - `TrackLocation::decode_from_le_bytes`
    /// - `process_track`
    ///
    /// # Assumptions
    /// - The `self.data` buffer stores raw G64 file contents.
    /// - The `G64HeaderInfo` structure is correctly formatted and interpreted.
    fn parse(&self) -> Option<ParsedData> {
        let hdr = G64HeaderInfo::decode_from_le_bytes(&self.data);
        assert_eq!(hdr.tag, [71, 67, 82, 45, 49, 53, 52, 49]);

        let track_count = (hdr.total_tracks / 2) as usize;
        let mut tracks = Vec::with_capacity(track_count * 2); // Pre-allocate for full and half tracks

        let header_size = size_of::<G64HeaderInfo>();
        let track_location_size = size_of::<TrackLocation>();
        let speedzone_base_offset = header_size + track_count * track_location_size;

        for i in 0..track_count {
            let offset = header_size + i * track_location_size;

            // Batch decode both track location and speed zone data
            let track_location = TrackLocation::decode_from_le_bytes(
                &self.data[offset..offset + track_location_size],
            );

            let speedzone_offset = speedzone_base_offset + i * track_location_size;
            let speedzone = TrackLocation::decode_from_le_bytes(
                &self.data[speedzone_offset..speedzone_offset + track_location_size],
            );

            // Process full track
            if let Some(mut track) = self.process_track(track_location.track_offset as usize) {
                track.speed_zone = speedzone.track_offset as i32;
                tracks.push(track);
            }

            // Process half track
            if let Some(mut track) = self.process_track(track_location.half_track_offset as usize) {
                track.speed_zone = speedzone.half_track_offset as i32;
                tracks.push(track);
            }
        }

        Some(ParsedData {
            g64_header: Some(hdr),
            tracks: if tracks.is_empty() {
                None
            } else {
                Some(tracks)
            },
        })
    }

    /// Parses data from the provided buffer and attempts to generate a `ParsedData` object.
    ///
    /// # Parameters
    /// - `data`: A reference to a `Vec<u8>` representing the buffer containing the data to be parsed.
    ///
    /// # Returns
    /// - `Some(ParsedData)`: If the data was successfully parsed into a `ParsedData` object.
    /// - `None`: If the parsing process fails.
    ///
    /// # Behavior
    /// - The method clones the provided buffer into the `self.data` field to store the current buffer data.
    /// - It then invokes the `parse` method to process the data.
    ///
    /// # Example
    /// ```
    /// let mut parser = Parser::new();
    /// let buffer = vec![1, 2, 3, 4];
    /// if let Some(parsed_data) = parser.parse_from_buffer(&buffer) {
    ///     println!("Successfully parsed data: {:?}", parsed_data);
    /// } else {
    ///     println!("Failed to parse data.");
    /// }
    /// ```
    ///
    /// # Notes
    /// - Cloning the buffer internally can be expensive for large datasets.
    /// - Ensure that the `parse` method is properly implemented to handle the expected format of `data`.
    ///
    pub fn parse_from_buffer(&mut self, data: &Vec<u8>) -> Option<ParsedData> {
        self.data = data.clone();
        self.parse()
    }

    ///
    /// Parses data from a file at the given file path and returns the parsed data.
    ///
    /// Calls the `load_file` method to read the file content and then processes the data using the `parse` method.
    ///
    /// # Arguments
    /// * `file_path` - A string slice holding the file path to read the data from.
    ///
    /// # Returns
    /// * `Some(ParsedData)` - If the file is successfully loaded and parsed.
    /// * `None` - If an error occurs while loading the file.
    ///
    /// # Example
    /// ```
    /// let mut parser = MyParser::new();
    /// if let Some(parsed_data) = parser.parse_from_file("data.txt") {
    ///     println!("Parsed data: {:?}", parsed_data);
    /// } else {
    ///     println!("Failed to parse file.");
    /// }
    /// ```
    pub fn parse_from_file(&mut self, file_path: &str) -> Option<ParsedData> {
        if self.load_file(file_path.to_string()).is_err() {
            return None;
        };
        self.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {}
}

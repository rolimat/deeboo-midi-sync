//! Platform layer over `CoreMIDI`: virtual ports whose packets carry host-time stamps in both
//! directions. On other platforms a stub keeps the crate compiling (Docker, CI lint).

#[cfg(target_os = "macos")]
pub use mac::{MidiClient, MidiIn, MidiOut};
#[cfg(not(target_os = "macos"))]
pub use stub::{MidiClient, MidiIn, MidiOut};

#[cfg(target_os = "macos")]
mod mac {
    use coremidi::{Client, PacketBuffer, PacketList, VirtualDestination, VirtualSource};

    /// One `CoreMIDI` client for the app's lifetime.
    pub struct MidiClient(Client);
    /// Dropping it removes the virtual destination.
    pub struct MidiIn(#[allow(dead_code)] VirtualDestination);
    /// Dropping it removes the virtual source.
    pub struct MidiOut(VirtualSource);

    impl MidiClient {
        /// # Errors
        /// The `OSStatus` of the failed `CoreMIDI` call.
        pub fn new(name: &str) -> Result<Self, String> {
            Client::new(name)
                .map(Self)
                .map_err(|s| format!("cannot create MIDI client (OSStatus {s})"))
        }

        /// Virtual destination; `on_packet` gets each packet's stamp (host ticks) and bytes.
        ///
        /// # Errors
        /// The `OSStatus` of the failed `CoreMIDI` call.
        pub fn open_input(
            &self,
            name: &str,
            mut on_packet: impl FnMut(u64, &[u8]) + Send + 'static,
        ) -> Result<MidiIn, String> {
            self.0
                .virtual_destination(name, move |list: &PacketList| {
                    for p in list {
                        on_packet(p.timestamp(), p.data());
                    }
                })
                .map(MidiIn)
                .map_err(|s| format!("cannot create virtual input '{name}' (OSStatus {s})"))
        }

        /// # Errors
        /// The `OSStatus` of the failed `CoreMIDI` call.
        pub fn open_output(&self, name: &str) -> Result<MidiOut, String> {
            self.0
                .virtual_source(name)
                .map(MidiOut)
                .map_err(|s| format!("cannot create virtual output '{name}' (OSStatus {s})"))
        }
    }

    impl MidiOut {
        /// One packet per `(host ticks, byte)`, all in a single `MIDIReceived` call.
        ///
        /// # Errors
        /// The `OSStatus` of the failed `CoreMIDI` call.
        pub fn send(&self, packets: &[(u64, u8)]) -> Result<(), String> {
            let Some((&(ts, byte), rest)) = packets.split_first() else {
                return Ok(());
            };
            let mut buf = PacketBuffer::new(ts, &[byte]);
            for &(ts, byte) in rest {
                buf.push_data(ts, &[byte]);
            }
            self.0
                .received(&buf)
                .map_err(|s| format!("MIDIReceived failed (OSStatus {s})"))
        }
    }
}

#[cfg(not(target_os = "macos"))]
#[allow(clippy::unused_self, clippy::needless_pass_by_value)] // keeps the real API shape
mod stub {
    const UNSUPPORTED: &str = "virtual MIDI ports need macOS";

    pub struct MidiClient;
    pub struct MidiIn;
    pub struct MidiOut;

    impl MidiClient {
        /// # Errors
        /// Always: there is no `CoreMIDI` here.
        pub fn new(_name: &str) -> Result<Self, String> {
            Err(UNSUPPORTED.into())
        }

        /// # Errors
        /// Always: there is no `CoreMIDI` here.
        pub fn open_input(
            &self,
            _name: &str,
            _on_packet: impl FnMut(u64, &[u8]) + Send + 'static,
        ) -> Result<MidiIn, String> {
            Err(UNSUPPORTED.into())
        }

        /// # Errors
        /// Always: there is no `CoreMIDI` here.
        pub fn open_output(&self, _name: &str) -> Result<MidiOut, String> {
            Err(UNSUPPORTED.into())
        }
    }

    impl MidiOut {
        /// # Errors
        /// Always: there is no `CoreMIDI` here.
        pub fn send(&self, _packets: &[(u64, u8)]) -> Result<(), String> {
            Err(UNSUPPORTED.into())
        }
    }
}

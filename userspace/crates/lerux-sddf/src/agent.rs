//! Tool argument page for the agent protection domain.
//!
//! LionsOS 0.4.0 has no agent header. Milestone 13 defines this page so `grok`
//! can pass a tool without a postcard message. The shell and the agent both
//! map it. Microkit zeroes the page. The shell fills [`agent_tool_t`] and
//! notifies the agent. The agent writes the result and notifies the shell.
//! Browse uses the same page plus the browse channel.

use crate::{fs_image, net_image};

/// One page. The tool struct sits at the start.
pub const AGENT_TOOL_VADDR: u64 = 0xa_000_000;
pub const AGENT_TOOL_SIZE: u64 = 0x1000;

/// Shell end of the tool channel. Notifying it starts the agent.
/// The agent completes a tool by notifying [`AGENT_TOOL_CHANNEL`].
pub const SHELL_TOOL_CHANNEL: u8 = 2;
/// Agent end of the tool channel.
pub const AGENT_TOOL_CHANNEL: u8 = 0;
/// Shell end of the browse channel. The agent notifies [`AGENT_BROWSE_CHANNEL`].
/// The shell writes the reply and notifies this end.
pub const SHELL_BROWSE_CHANNEL: u8 = 3;
/// Agent end of the browse channel.
pub const AGENT_BROWSE_CHANNEL: u8 = 1;

/// The agent is the filesystem client. The channel id matches the shell client page.
pub const AGENT_FS_CHANNEL: u8 = fs_image::FS_SHELL_SERVER_CHANNEL;
/// The agent is the network client. These ids match the shell network page.
pub const AGENT_NET_RX_CHANNEL: u8 = net_image::SHELL_NET_RX_CHANNEL;
pub const AGENT_NET_TX_CHANNEL: u8 = net_image::SHELL_NET_TX_CHANNEL;

pub const AGENT_KIND_READ: u32 = 1;
pub const AGENT_KIND_WRITE: u32 = 2;
pub const AGENT_KIND_EDIT: u32 = 3;
pub const AGENT_KIND_LIST_DIR: u32 = 4;
pub const AGENT_KIND_SEARCH: u32 = 5;
pub const AGENT_KIND_WEB_FETCH: u32 = 6;
pub const AGENT_KIND_BROWSE: u32 = 7;

/// The page is idle. Microkit zeroes the region, so boot starts here.
pub const AGENT_STATE_IDLE: u32 = 0;
/// The agent is waiting for the shell to answer Browse.
pub const AGENT_STATE_BROWSE: u32 = 1;
/// The shell has written the Browse reply.
pub const AGENT_STATE_BROWSE_DONE: u32 = 2;
/// The tool finished and `result` holds the bytes.
pub const AGENT_STATE_OK: u32 = 3;
/// The tool failed. `result_len` is 0.
pub const AGENT_STATE_ERR: u32 = 4;

pub const AGENT_NAME_CAP: usize = 64;
pub const AGENT_ARG_CAP: usize = 128;
pub const AGENT_RESULT_CAP: usize = 256;

/// Arguments and result for one tool. The fields are plain integers so the
/// page has a C layout. There is no postcard encoding.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct agent_tool_t {
    pub kind: u32,
    pub name_len: u32,
    pub arg_len: u32,
    pub name: [u8; AGENT_NAME_CAP],
    pub arg: [u8; AGENT_ARG_CAP],
    pub state: u32,
    pub result_len: u32,
    pub result: [u8; AGENT_RESULT_CAP],
}

const _: () = assert!(core::mem::size_of::<agent_tool_t>() <= AGENT_TOOL_SIZE as usize);
const _: () = assert!(core::mem::align_of::<agent_tool_t>() <= 8);

/// A zeroed tool page. Padding stays zero because every field is assigned below.
pub fn agent_tool_zeroed() -> agent_tool_t {
    // SAFETY: every field is an integer or a byte array.
    unsafe { core::mem::zeroed() }
}

/// The bytes `len` describes, when they sit inside `buf`.
pub fn agent_bytes(buf: &[u8], len: u32) -> Option<&[u8]> {
    let len = usize::try_from(len).ok()?;
    buf.get(..len)
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, offset_of, size_of};

    use super::*;

    #[test]
    fn tool_header_fits_in_one_page() {
        assert!(size_of::<agent_tool_t>() <= AGENT_TOOL_SIZE as usize);
        assert!(align_of::<agent_tool_t>() <= 8);
        assert_eq!(offset_of!(agent_tool_t, kind), 0);
        assert_eq!(
            offset_of!(agent_tool_t, state),
            4 + 4 + 4 + AGENT_NAME_CAP + AGENT_ARG_CAP
        );
        let tool = agent_tool_zeroed();
        assert_eq!(tool.kind, 0);
        assert_eq!(tool.state, AGENT_STATE_IDLE);
        assert_eq!(AGENT_FS_CHANNEL, 2);
        assert_eq!(AGENT_NET_RX_CHANNEL, 3);
        assert_eq!(AGENT_NET_TX_CHANNEL, 4);
    }
}

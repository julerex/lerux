//! Tool argument page for the agent protection domain.
//!
//! `LionsOS` 0.4.0 has no agent header. Milestone 13 defines this page so `grok`
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

#[expect(
    clippy::cast_possible_truncation,
    reason = "the tool page is 0x1000 bytes and fits in usize"
)]
const _: () = assert!(core::mem::size_of::<agent_tool_t>() <= AGENT_TOOL_SIZE as usize);
const _: () = assert!(core::mem::align_of::<agent_tool_t>() <= 8);

/// A zeroed tool page. Padding stays zero because every field is assigned below.
#[must_use]
pub fn agent_tool_zeroed() -> agent_tool_t {
    // SAFETY: every field is an integer or a byte array.
    unsafe { core::mem::zeroed() }
}

/// The bytes `len` describes, when they sit inside `buf`.
#[must_use]
pub fn agent_bytes(buf: &[u8], len: u32) -> Option<&[u8]> {
    let len = usize::try_from(len).ok()?;
    buf.get(..len)
}

/// The agent is waiting for the shell to answer Browse.
///
/// `sddf-agent` `browse` does this, then notifies [`AGENT_BROWSE_CHANNEL`] and waits.
pub fn agent_mark_browse(tool: &mut agent_tool_t) {
    tool.state = AGENT_STATE_BROWSE;
}

/// Write the Browse reply. The shell notifies the agent only when this returns true.
///
/// A page that is not a Browse request is left unchanged. A name that does not fit
/// in the name field or the result field finishes with `result_len` 0. Otherwise
/// the result is the name.
#[must_use]
pub fn agent_answer_browse(tool: &mut agent_tool_t) -> bool {
    if tool.state != AGENT_STATE_BROWSE || tool.kind != AGENT_KIND_BROWSE {
        return false;
    }
    let name_len = usize::try_from(tool.name_len).unwrap_or(usize::MAX);
    if name_len > tool.name.len() || name_len > tool.result.len() {
        tool.state = AGENT_STATE_BROWSE_DONE;
        tool.result_len = 0;
        return true;
    }
    tool.result[..name_len].copy_from_slice(&tool.name[..name_len]);
    tool.result_len = u32::try_from(name_len).unwrap_or(u32::MAX);
    tool.state = AGENT_STATE_BROWSE_DONE;
    true
}

/// The shell copied a name into the result.
#[must_use]
pub fn agent_browse_accepted(tool: &agent_tool_t) -> bool {
    tool.state == AGENT_STATE_BROWSE_DONE && tool.result_len > 0
}

/// Record the worker's outcome. Failure clears the result length.
pub fn agent_store_outcome(tool: &mut agent_tool_t, ok: bool) {
    if ok {
        tool.state = AGENT_STATE_OK;
    } else {
        tool.result_len = 0;
        tool.state = AGENT_STATE_ERR;
    }
}

#[cfg(test)]
mod tests {
    use core::mem::{align_of, offset_of, size_of};

    use super::*;

    #[test]
    fn tool_header_fits_in_one_page() {
        assert!(
            size_of::<agent_tool_t>() <= usize::try_from(AGENT_TOOL_SIZE).expect("tool page fits")
        );
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

    fn browse_page(name: &[u8], name_len: u32) -> agent_tool_t {
        let mut tool = agent_tool_zeroed();
        tool.kind = AGENT_KIND_BROWSE;
        tool.state = AGENT_STATE_BROWSE;
        tool.name_len = name_len;
        let n = name.len().min(tool.name.len());
        tool.name[..n].copy_from_slice(&name[..n]);
        tool
    }

    #[test]
    fn answer_copies_a_name_that_fits() {
        let mut tool = browse_page(b"host", 4);
        assert!(agent_answer_browse(&mut tool));
        assert_eq!(tool.state, AGENT_STATE_BROWSE_DONE);
        assert_eq!(tool.result_len, 4);
        assert_eq!(&tool.result[..4], b"host");
        assert!(agent_browse_accepted(&tool));
        agent_store_outcome(&mut tool, true);
        assert_eq!(tool.state, AGENT_STATE_OK);
        assert_eq!(tool.result_len, 4);
    }

    #[test]
    fn answer_rejects_a_name_longer_than_the_field() {
        let mut tool = browse_page(b"host", 65);
        assert!(agent_answer_browse(&mut tool));
        assert_eq!(tool.state, AGENT_STATE_BROWSE_DONE);
        assert_eq!(tool.result_len, 0);
        assert!(!agent_browse_accepted(&tool));
        agent_store_outcome(&mut tool, false);
        assert_eq!(tool.state, AGENT_STATE_ERR);
        assert_eq!(tool.result_len, 0);
    }

    #[test]
    fn answer_leaves_a_page_that_is_not_browse() {
        let mut tool = agent_tool_zeroed();
        tool.kind = AGENT_KIND_READ;
        tool.state = AGENT_STATE_IDLE;
        assert!(!agent_answer_browse(&mut tool));
        assert_eq!(tool.state, AGENT_STATE_IDLE);
        assert_eq!(tool.result_len, 0);
    }
}

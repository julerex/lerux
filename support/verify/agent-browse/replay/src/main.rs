//! Same steps as `AgentBrowse.tla`.
//!
//! The page updates call `agent_mark_browse`, `agent_answer_browse`,
//! `agent_browse_accepted`, and `agent_store_outcome`. Whether a notify is
//! still in the protection domain is read from that source. A branch that
//! moves has to change this file and the TLA module together.

use std::{
    collections::{HashMap, HashSet, VecDeque},
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use lerux_sddf::{
    agent_answer_browse, agent_browse_accepted, agent_mark_browse, agent_store_outcome,
    agent_tool_t, agent_tool_zeroed, AGENT_KIND_BROWSE, AGENT_STATE_ERR, AGENT_STATE_OK,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Run {
    ShellW,
    AgentW,
    AgentEnter,
    AgentBody,
    ShellEnter,
    ShellBody,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum ShellPhase {
    Notify,
    Wait,
    Read,
    Done,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum AgentPhase {
    Tool,
    Mark,
    Signal,
    Bwait,
    Check,
    Store,
    Tsignal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Resume {
    None,
    ShellW,
    ShellBody,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct World {
    state: u32,
    result_len: u32,
    bad_bytes: bool,
    shell_phase: ShellPhase,
    shell_blocked: bool,
    shell_tool_sem: bool,
    agent_phase: AgentPhase,
    agent_blocked: bool,
    agent_tool_sem: bool,
    agent_browse_sem: bool,
    ntfn_agent_tool: bool,
    ntfn_agent_browse: bool,
    ntfn_shell_browse: bool,
    ntfn_shell_tool: bool,
    seen_agent_tool: bool,
    seen_agent_browse: bool,
    seen_shell_browse: bool,
    seen_shell_tool: bool,
    resume: Resume,
    run: Run,
}

struct Kind {
    name_len: u32,
    name: [u8; 64],
    expect_ok: bool,
}

#[derive(Clone, Copy)]
struct Shape {
    answer_notify: bool,
    browse_notifies: bool,
    worker_notifies: bool,
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let cmd = args.next();
    let which = args.next();
    let Some(kind) = which.as_deref().and_then(Kind::parse) else {
        eprintln!("usage: agent-browse probe|check fits|reject");
        return ExitCode::from(2);
    };
    let shape = match source_shape(&repo_root()) {
        Ok(shape) => shape,
        Err(error) => {
            println!("error={error}");
            return ExitCode::from(2);
        }
    };
    match cmd.as_deref() {
        Some("probe") => {
            print_probe(&kind, shape);
            ExitCode::SUCCESS
        }
        Some("check") => check(&kind, shape),
        _ => {
            eprintln!("usage: agent-browse probe|check fits|reject");
            ExitCode::from(2)
        }
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../..")
}

impl Kind {
    fn parse(name: &str) -> Option<Self> {
        let mut name_bytes = [0; 64];
        match name {
            "fits" => {
                name_bytes[..4].copy_from_slice(b"host");
                Some(Self {
                    name_len: 4,
                    name: name_bytes,
                    expect_ok: true,
                })
            }
            "reject" => Some(Self {
                name_len: 65,
                name: name_bytes,
                expect_ok: false,
            }),
            _ => None,
        }
    }
}

fn fresh(kind: &Kind) -> agent_tool_t {
    let mut tool = agent_tool_zeroed();
    tool.kind = AGENT_KIND_BROWSE;
    tool.name_len = kind.name_len;
    let n = usize::min(kind.name_len as usize, tool.name.len());
    tool.name[..n].copy_from_slice(&kind.name[..n]);
    tool
}

fn print_probe(kind: &Kind, shape: Shape) {
    let mut tool = fresh(kind);
    agent_mark_browse(&mut tool);
    let returned = agent_answer_browse(&mut tool);
    let reply_state = tool.state;
    let reply_len = tool.result_len;
    let accepted = agent_browse_accepted(&tool);
    agent_store_outcome(&mut tool, accepted);
    println!("do_notify={}", returned && shape.answer_notify);
    println!("browse_notifies={}", shape.browse_notifies);
    println!("worker_notifies={}", shape.worker_notifies);
    println!("reply_state={reply_state}");
    println!("reply_len={reply_len}");
    println!("final_state={}", tool.state);
    println!("final_len={}", tool.result_len);
    println!("expect_ok={}", kind.expect_ok);
    println!("name_len={}", kind.name_len);
}

fn check(kind: &Kind, shape: Shape) -> ExitCode {
    match explore(kind, shape) {
        Explore::Ok { states, trace } => {
            println!("states={states}");
            println!("trace={trace}");
            println!("verdict=ok");
            ExitCode::SUCCESS
        }
        Explore::Stuck { trace } => {
            println!("trace={trace}");
            println!("verdict=stuck");
            ExitCode::from(1)
        }
        Explore::Result { trace } => {
            println!("trace={trace}");
            println!("verdict=result");
            ExitCode::from(1)
        }
        Explore::Blowup => {
            println!("error=more than 10000 states");
            ExitCode::from(2)
        }
    }
}

enum Explore {
    Ok { states: usize, trace: String },
    Stuck { trace: String },
    Result { trace: String },
    Blowup,
}

fn explore(kind: &Kind, shape: Shape) -> Explore {
    let init = World::init();
    let mut seen = HashSet::new();
    let mut prev: HashMap<World, (&'static str, World)> = HashMap::new();
    let mut queue = VecDeque::new();
    seen.insert(init);
    queue.push_back(init);
    let mut done = Vec::new();
    let mut stuck = Vec::new();
    while let Some(world) = queue.pop_front() {
        if seen.len() > 10_000 {
            return Explore::Blowup;
        }
        let next = successors(&world, kind, shape);
        if next.is_empty() {
            if world.shell_phase == ShellPhase::Done {
                done.push(world);
            } else {
                stuck.push(world);
            }
            continue;
        }
        for (name, next_world) in next {
            if seen.insert(next_world) {
                prev.insert(next_world, (name, world));
                queue.push_back(next_world);
            }
        }
    }
    if let Some(world) = stuck.first().copied() {
        return Explore::Stuck {
            trace: trace_of(&prev, world),
        };
    }
    let mut good = None;
    for world in done {
        if page_ok(&world, kind) {
            good = Some(world);
        } else {
            return Explore::Result {
                trace: trace_of(&prev, world),
            };
        }
    }
    match good {
        Some(world) => Explore::Ok {
            states: seen.len(),
            trace: trace_of(&prev, world),
        },
        None => Explore::Stuck {
            trace: String::new(),
        },
    }
}

fn trace_of(prev: &HashMap<World, (&'static str, World)>, end: World) -> String {
    let mut steps = Vec::new();
    let mut cur = end;
    while let Some((name, parent)) = prev.get(&cur) {
        steps.push(*name);
        cur = *parent;
    }
    steps.reverse();
    steps.join(" ")
}

fn page_ok(world: &World, kind: &Kind) -> bool {
    if world.bad_bytes {
        return false;
    }
    if kind.expect_ok {
        world.state == AGENT_STATE_OK && world.result_len == kind.name_len
    } else {
        world.state == AGENT_STATE_ERR && world.result_len == 0
    }
}

fn successors(world: &World, kind: &Kind, shape: Shape) -> Vec<(&'static str, World)> {
    let mut out = Vec::new();
    push(&mut out, "ShellNotify", shell_notify(world));
    push(&mut out, "AgentEnter", agent_enter(world));
    push(&mut out, "AgentRecvTool", agent_recv_tool(world));
    push(&mut out, "AgentRecvBrowse", agent_recv_browse(world));
    push(&mut out, "AgentLeave", agent_leave(world));
    push(&mut out, "AgentMark", agent_mark(world));
    push(&mut out, "AgentSignal", agent_signal(world, shape));
    push(&mut out, "AgentWaitBrowse", agent_wait_browse(world));
    push(&mut out, "AgentCheck", agent_check(world));
    push(&mut out, "AgentStore", agent_store(world, kind));
    push(&mut out, "AgentSignalTool", agent_signal_tool(world, shape));
    push(&mut out, "AgentWaitTool", agent_wait_tool(world));
    push(&mut out, "ShellWaitOk", shell_wait(world));
    push(&mut out, "ShellEnter", shell_enter(world));
    push(&mut out, "ShellAnswer", shell_answer(world, kind, shape));
    push(&mut out, "ShellRecvTool", shell_recv_tool(world));
    push(&mut out, "ShellLeave", shell_leave(world));
    push(&mut out, "ShellRead", shell_read(world));
    out
}

fn push(out: &mut Vec<(&'static str, World)>, name: &'static str, next: Option<World>) {
    if let Some(next) = next {
        out.push((name, next));
    }
}

impl World {
    fn init() -> Self {
        Self {
            state: 0,
            result_len: 0,
            bad_bytes: false,
            shell_phase: ShellPhase::Notify,
            shell_blocked: false,
            shell_tool_sem: false,
            agent_phase: AgentPhase::Tool,
            agent_blocked: true,
            agent_tool_sem: false,
            agent_browse_sem: false,
            ntfn_agent_tool: false,
            ntfn_agent_browse: false,
            ntfn_shell_browse: false,
            ntfn_shell_tool: false,
            seen_agent_tool: false,
            seen_agent_browse: false,
            seen_shell_browse: false,
            seen_shell_tool: false,
            resume: Resume::None,
            run: Run::ShellW,
        }
    }

    fn tool(&self, kind: &Kind) -> agent_tool_t {
        let mut tool = fresh(kind);
        tool.state = self.state;
        tool.result_len = self.result_len;
        tool
    }
}

fn shell_notify(world: &World) -> Option<World> {
    if world.run != Run::ShellW || world.shell_phase != ShellPhase::Notify || world.shell_blocked {
        return None;
    }
    let mut next = *world;
    next.ntfn_agent_tool = true;
    next.shell_phase = ShellPhase::Wait;
    next.resume = Resume::ShellW;
    next.run = Run::AgentEnter;
    Some(next)
}

fn agent_enter(world: &World) -> Option<World> {
    if world.run != Run::AgentEnter {
        return None;
    }
    let mut next = *world;
    next.seen_agent_tool = world.ntfn_agent_tool;
    next.seen_agent_browse = world.ntfn_agent_browse;
    next.ntfn_agent_tool = false;
    next.ntfn_agent_browse = false;
    next.run = Run::AgentBody;
    Some(next)
}

fn agent_recv_tool(world: &World) -> Option<World> {
    if world.run != Run::AgentBody || !world.seen_agent_tool {
        return None;
    }
    let mut next = *world;
    next.seen_agent_tool = false;
    if world.agent_tool_sem {
        return Some(next);
    }
    if world.agent_blocked && world.agent_phase == AgentPhase::Tool {
        next.agent_blocked = false;
        next.agent_phase = AgentPhase::Mark;
        next.run = Run::AgentW;
    } else {
        next.agent_tool_sem = true;
    }
    Some(next)
}

fn agent_recv_browse(world: &World) -> Option<World> {
    if world.run != Run::AgentBody || world.seen_agent_tool || !world.seen_agent_browse {
        return None;
    }
    let mut next = *world;
    next.seen_agent_browse = false;
    if world.agent_browse_sem {
        return Some(next);
    }
    if world.agent_blocked && world.agent_phase == AgentPhase::Bwait {
        next.agent_blocked = false;
        next.agent_phase = AgentPhase::Check;
        next.run = Run::AgentW;
    } else {
        next.agent_browse_sem = true;
    }
    Some(next)
}

fn agent_leave(world: &World) -> Option<World> {
    if world.run != Run::AgentBody || world.seen_agent_tool || world.seen_agent_browse {
        return None;
    }
    let mut next = *world;
    if world.ntfn_agent_tool || world.ntfn_agent_browse {
        next.run = Run::AgentEnter;
    } else if world.resume == Resume::ShellW {
        next.run = Run::ShellW;
        next.resume = Resume::None;
    } else if world.resume == Resume::ShellBody {
        next.run = Run::ShellBody;
        next.resume = Resume::None;
    } else {
        next.run = Run::ShellEnter;
    }
    Some(next)
}

fn agent_mark(world: &World) -> Option<World> {
    if world.run != Run::AgentW || world.agent_phase != AgentPhase::Mark || world.agent_blocked {
        return None;
    }
    let mut next = *world;
    let mut tool = agent_tool_zeroed();
    tool.state = world.state;
    agent_mark_browse(&mut tool);
    next.state = tool.state;
    next.agent_phase = AgentPhase::Signal;
    Some(next)
}

fn agent_signal(world: &World, shape: Shape) -> Option<World> {
    if world.run != Run::AgentW || world.agent_phase != AgentPhase::Signal || world.agent_blocked {
        return None;
    }
    let mut next = *world;
    if shape.browse_notifies {
        next.ntfn_shell_browse = true;
    }
    next.agent_phase = AgentPhase::Bwait;
    Some(next)
}

fn agent_wait_browse(world: &World) -> Option<World> {
    if world.run != Run::AgentW || world.agent_phase != AgentPhase::Bwait || world.agent_blocked {
        return None;
    }
    let mut next = *world;
    if world.agent_browse_sem {
        next.agent_browse_sem = false;
        next.agent_phase = AgentPhase::Check;
    } else {
        next.agent_blocked = true;
        next.run = Run::AgentBody;
    }
    Some(next)
}

fn agent_check(world: &World) -> Option<World> {
    if world.run != Run::AgentW || world.agent_phase != AgentPhase::Check || world.agent_blocked {
        return None;
    }
    let mut next = *world;
    next.agent_phase = AgentPhase::Store;
    Some(next)
}

fn agent_store(world: &World, kind: &Kind) -> Option<World> {
    if world.run != Run::AgentW || world.agent_phase != AgentPhase::Store || world.agent_blocked {
        return None;
    }
    let mut next = *world;
    let mut tool = world.tool(kind);
    let accepted = agent_browse_accepted(&tool);
    agent_store_outcome(&mut tool, accepted);
    next.state = tool.state;
    next.result_len = tool.result_len;
    next.agent_phase = AgentPhase::Tsignal;
    Some(next)
}

fn agent_signal_tool(world: &World, shape: Shape) -> Option<World> {
    if world.run != Run::AgentW || world.agent_phase != AgentPhase::Tsignal || world.agent_blocked {
        return None;
    }
    let mut next = *world;
    if shape.worker_notifies {
        next.ntfn_shell_tool = true;
    }
    next.agent_phase = AgentPhase::Tool;
    Some(next)
}

fn agent_wait_tool(world: &World) -> Option<World> {
    if world.run != Run::AgentW || world.agent_phase != AgentPhase::Tool || world.agent_blocked {
        return None;
    }
    let mut next = *world;
    if world.agent_tool_sem {
        next.agent_tool_sem = false;
        next.agent_phase = AgentPhase::Mark;
    } else {
        next.agent_blocked = true;
        next.run = Run::AgentBody;
    }
    Some(next)
}

fn shell_wait(world: &World) -> Option<World> {
    if world.run != Run::ShellW || world.shell_phase != ShellPhase::Wait || world.shell_blocked {
        return None;
    }
    let mut next = *world;
    if world.shell_tool_sem {
        next.shell_tool_sem = false;
        next.shell_phase = ShellPhase::Read;
    } else {
        next.shell_blocked = true;
        next.run = Run::ShellEnter;
    }
    Some(next)
}

fn shell_enter(world: &World) -> Option<World> {
    if world.run != Run::ShellEnter || (!world.ntfn_shell_browse && !world.ntfn_shell_tool) {
        return None;
    }
    let mut next = *world;
    next.seen_shell_browse = world.ntfn_shell_browse;
    next.seen_shell_tool = world.ntfn_shell_tool;
    next.ntfn_shell_browse = false;
    next.ntfn_shell_tool = false;
    next.run = Run::ShellBody;
    Some(next)
}

fn shell_answer(world: &World, kind: &Kind, shape: Shape) -> Option<World> {
    if world.run != Run::ShellBody || !world.seen_shell_browse {
        return None;
    }
    let mut next = *world;
    next.seen_shell_browse = false;
    let mut tool = world.tool(kind);
    let returned = agent_answer_browse(&mut tool);
    next.state = tool.state;
    next.result_len = tool.result_len;
    if kind.expect_ok && returned {
        let n = kind.name_len as usize;
        if tool.result_len != kind.name_len || tool.result[..n] != kind.name[..n] {
            next.bad_bytes = true;
        }
    }
    if returned && shape.answer_notify {
        next.ntfn_agent_browse = true;
        next.resume = Resume::ShellBody;
        next.run = Run::AgentEnter;
    }
    Some(next)
}

fn shell_recv_tool(world: &World) -> Option<World> {
    if world.run != Run::ShellBody || world.seen_shell_browse || !world.seen_shell_tool {
        return None;
    }
    let mut next = *world;
    next.seen_shell_tool = false;
    if world.shell_tool_sem {
        return Some(next);
    }
    if world.shell_blocked && world.shell_phase == ShellPhase::Wait {
        next.shell_blocked = false;
        next.shell_phase = ShellPhase::Read;
        next.run = Run::ShellW;
    } else {
        next.shell_tool_sem = true;
    }
    Some(next)
}

fn shell_leave(world: &World) -> Option<World> {
    if world.run != Run::ShellBody || world.seen_shell_browse || world.seen_shell_tool {
        return None;
    }
    let mut next = *world;
    next.run = Run::ShellEnter;
    Some(next)
}

fn shell_read(world: &World) -> Option<World> {
    if world.run != Run::ShellW || world.shell_phase != ShellPhase::Read || world.shell_blocked {
        return None;
    }
    let mut next = *world;
    next.shell_phase = ShellPhase::Done;
    Some(next)
}

fn source_shape(root: &Path) -> Result<Shape, String> {
    let grok = read_fn(
        &root.join("userspace/pds/sddf-grok/src/main.rs"),
        "answer_browse",
    )?;
    let notified = read_fn(
        &root.join("userspace/pds/sddf-grok/src/main.rs"),
        "notified",
    )?;
    let browse = read_fn(
        &root.join("userspace/pds/sddf-agent/src/tools.rs"),
        "browse",
    )?;
    let worker = read_fn(&root.join("userspace/pds/sddf-agent/src/main.rs"), "worker")?;
    shell_order(&notified)?;
    Ok(Shape {
        answer_notify: answer_notify(&grok)?,
        browse_notifies: browse_notify(&browse)?,
        worker_notifies: worker_notify(&worker)?,
    })
}

fn read_fn(path: &Path, name: &str) -> Result<String, String> {
    let src = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    fn_body(&src, name).ok_or_else(|| format!("{} has no fn {name}", path.display()))
}

fn fn_body(src: &str, name: &str) -> Option<String> {
    let needle = format!("fn {name}(");
    let start = src.find(&needle)?;
    let rest = &src[start..];
    let brace = rest.find('{')?;
    let mut depth = 0usize;
    for (index, ch) in rest[brace..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(rest[brace..=brace + index].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

fn answer_notify(body: &str) -> Result<bool, String> {
    let call = body
        .find("agent_answer_browse")
        .ok_or("answer_browse does not call agent_answer_browse")?;
    Ok(notify_between(body, call, usize::MAX, "BROWSE_CH"))
}

fn browse_notify(body: &str) -> Result<bool, String> {
    let mark = body
        .find("agent_mark_browse")
        .ok_or("browse does not call agent_mark_browse")?;
    let wait = body
        .find("microkit_cothread_wait_on_channel")
        .ok_or("browse does not wait")?;
    let accepted = body
        .find("agent_browse_accepted")
        .ok_or("browse does not call agent_browse_accepted")?;
    if mark >= wait || wait >= accepted {
        return Err("browse is not mark, then wait, then accepted".to_string());
    }
    Ok(notify_between(body, mark, wait, "AGENT_BROWSE_CHANNEL"))
}

fn worker_notify(body: &str) -> Result<bool, String> {
    let store = body
        .find("agent_store_outcome")
        .ok_or("worker does not call agent_store_outcome")?;
    let notify = match body[store..].find(".notify()") {
        Some(index) => store + index,
        None => return Ok(false),
    };
    if body[store..notify].contains("wait_on_channel") {
        return Ok(false);
    }
    Ok(body[store..notify].contains("TOOL_CH"))
}

fn notify_between(body: &str, from: usize, before: usize, channel: &str) -> bool {
    let Some(index) = body[from..].find(".notify()") else {
        return false;
    };
    let notify = from + index;
    notify < before && body[from..notify].contains(channel)
}

fn shell_order(body: &str) -> Result<(), String> {
    let answer = body
        .find("answer_browse()")
        .ok_or("shell notified does not call answer_browse")?;
    let tool = body
        .find("microkit_cothread_recv_ntfn(TOOL_CH)")
        .ok_or("shell notified does not receive the tool channel")?;
    if answer > tool {
        return Err(
            "shell notified handles the tool channel before answer_browse; update the model"
                .to_string(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use lerux_sddf::{AGENT_STATE_BROWSE, AGENT_STATE_BROWSE_DONE};

    use super::*;

    #[test]
    fn state_numbers_match_the_model() {
        assert_eq!(AGENT_STATE_BROWSE, 1);
        assert_eq!(AGENT_STATE_BROWSE_DONE, 2);
        assert_eq!(AGENT_STATE_OK, 3);
        assert_eq!(AGENT_STATE_ERR, 4);
    }

    #[test]
    fn a_notify_after_the_answer_call_is_a_site() {
        let body =
            "fn answer_browse() { agent_answer_browse(tool); Channel::new(BROWSE_CH).notify(); }";
        assert!(answer_notify(body).unwrap());
    }

    #[test]
    fn a_missing_notify_is_not_a_site() {
        let body = "fn answer_browse() { let _ = agent_answer_browse(tool); }";
        assert!(!answer_notify(body).unwrap());
    }

    #[test]
    fn protection_domain_sources_still_parse() {
        assert!(source_shape(&repo_root()).is_ok());
    }
}

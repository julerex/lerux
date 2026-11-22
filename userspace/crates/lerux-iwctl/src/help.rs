//! `iwctl help` and `iwctl --help`, from iwd 3.12 `command_display_help`.

use crate::{
    fmt::{caption, footer, header, row, Align, Col, Out},
    table::SPECS,
};

pub(crate) fn render(out: &mut Out<'_>, interactive: bool) {
    usage(out);
    options(out);
    commands(out);
    if interactive {
        misc(out);
    }
}

fn usage(out: &mut Out<'_>) {
    out.nl();
    caption(out, b"iwctl version 3.12");
    header(
        out,
        &[Col {
            text: b"Usage",
            width: 5,
            align: Align::Left,
        }],
    );
    rule_close(out);
    out.push(b"  iwctl [--options] [commands]\n");
    footer(out);
}

fn options(out: &mut Out<'_>) {
    caption(out, b"Available options");
    header(out, &option_cols());
    rule_close(out);
    option(out, b"username", b"Provide username\n");
    option(out, b"password", b"Provide password\n");
    option(out, b"passphrase", b"Provide passphrase\n");
    option(
        out,
        b"dont-ask",
        b"Don't ask for missing\n\t\t\t\t\t\t      credentials\n",
    );
    option(out, b"help", b"Display help\n");
    footer(out);
}

fn option_cols() -> [Col<'static>; 2] {
    [
        Col {
            text: b"Options",
            width: 50,
            align: Align::Left,
        },
        Col {
            text: b"Description",
            width: 28,
            align: Align::Left,
        },
    ]
}

fn option(out: &mut Out<'_>, name: &[u8], desc: &[u8]) {
    out.push(b"  --");
    out.push(name);
    out.spaces(48usize.saturating_sub(name.len()));
    out.push(b"  ");
    out.push(desc);
}

fn commands(out: &mut Out<'_>) {
    caption(out, b"Available commands");
    header(out, &option_cols_named(b"Commands"));
    rule_close(out);
    for spec in SPECS {
        if spec.family.is_empty() {
            continue;
        }
        if !spec.caption.is_empty() {
            out.nl();
            out.push(spec.caption);
            out.push(b":\n");
        }
        if spec.desc.is_empty() {
            continue;
        }
        row(out, &[(spec.line, 50), (spec.desc, 30)]);
    }
    footer(out);
}

fn option_cols_named(name: &'static [u8]) -> [Col<'static>; 2] {
    [
        Col {
            text: name,
            width: 50,
            align: Align::Left,
        },
        Col {
            text: b"Description",
            width: 28,
            align: Align::Left,
        },
    ]
}

fn misc(out: &mut Out<'_>) {
    out.push(b"\nMiscellaneous:\n");
    for spec in SPECS {
        if spec.family.is_empty() && !spec.desc.is_empty() {
            row(out, &[(spec.line, 50), (spec.desc, 30)]);
        }
    }
}

fn rule_close(out: &mut Out<'_>) {
    crate::fmt::rule(out);
}

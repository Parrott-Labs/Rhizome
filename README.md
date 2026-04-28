<img width="1267" height="462" alt="Rhizome logo" src="https://github.com/user-attachments/assets/a8844151-2b87-446e-bb16-318d11a0b16b" />

# Rhizome - the CRM that lives inside your terminal

A **local-first**, TUI for managing clients and projects.

## At a glance

```
┌─ rhizome ──────────────────────────────────────────────── 120×36 ─┐
│                                                                   │
│  ≋ RHIZOME v0.1.0    [1] dashboard · [2] clients · [3] projects   │
│                                                                   │
│  ─────────────────────────────────────────────────────────────    │
│                                                                   │
│   CLIENTS           ACTIVE            HOURS LOGGED    ATTENTION   │
│   12                7                 31.5h           2           │
│                     of 23 total       +4.2h vs last   follow-up   │
│                                                                   │
│   RECENT ACTIVITY                      NETWORK · ACTIVE RHIZOME   │
│   ─────────────────────────────        ─────────────────────────  │
│   │ halyard   north & co.  ● active    ≋ rhizome                  │
│     ember     claywork     ● active    ├── ◉ north & co.          │
│     lattice   studio kw.   ● blocked   │   ├── halyard  ● pending │
│     bloom     horus        ● pending   │   └── atlas    ○ active  │
│     osprey    zilt         · archived  ├── ◉ claywork             │
│                                        │   └── ember    ● active  │
│  ─────────────────────────────────────────────────────────────    │
│  ── NORMAL ──   j k 123 gg G   / : q                ● online      │
└───────────────────────────────────────────────────────────────────┘
```

---

## Features

| | |
|---|---|
| **local‑first**      | Locally saved TOML files live on your drive. |
| **vim‑native**       | `hjkl`, `:commands`, `/search`. modal. no mouse needed. |
| **bare‑metal**       | No electron, no chromium, no telemetry. just a binary |
| **minimalistic interface**       | Ships with projects, clients, hours and contact logs |

---

## Install
Download the binary from the releases tab, optionally add an alias.

Or build it yourself:

```
cargo build
./target/debug/rhizome
```

## Usage

### Keys
| key           | in                  | does                           |
|---------------|---------------------|--------------------------------|
| `1` `2` `3`   | anywhere            | dashboard / clients / projects |
| `c`           | client list         | opens up contact log sidepanel | 
| `l`           | project list        | opens up hour log sidepanel    |
| `j` `k`       | list                | move selection down / up       |
| `gg` `G`      | list                | first / last                   |
| `/`           | anywhere            | live filter                    |
| `n`           | list                | new                            |
| `e`           | row / detail        | edit                           |
| `d`           | row                 | delete (confirms)              |
| `q`  `:q`     | anywhere            | quit                           |
| `Esc`         | anywhere            | back to NORMAL                 |

## Status

This project is currently undergoing frequent updates. And will be considered unstable before version 1.0. Make sure you back up your database (`.local/share/rhizome`).

### Future plans
- Documentation
- Client notes and project notes
- Export functionality
- Paid sync and cloud service
- Quick edit cli support
- Proper installation
- Config file

# Rhizome - the CRM that lives inside your terminal

This CRM is under consistent development and is currently in a very early proof of concept. If you use this make sure to always have a back-up of your database.

## Features

### Global
- Local file first mentality, you own your data
- Fully offline capability
- Vim like keybindings
- Lives inside your terminal and as files in your directory

### Dashboard
- A quick glance for current status of projects and clients that need contacting

### Clients
- Saving of detailed information of clients
- Warning when contact with client has stopped for a while
- Multiple projects per client

### Projects
- Budget features
- Status
- Manually add hours

### Future
- Documentation
- Client notes and project notes
- Export functionality
- Paid sync and cloud service

## Install it yourself
It's currently only tested on Linux. You need Rust (and cargo) installed to build it yourself.

```
cargo build
./target/debug/rhizome
```

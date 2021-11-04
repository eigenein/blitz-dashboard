[![Last commit](https://img.shields.io/github/last-commit/eigenein/blitz-dashboard?logo=github)](https://github.com/eigenein/blitz-dashboard/commits/master)
[![Build status](https://github.com/eigenein/blitz-dashboard/actions/workflows/check.yml/badge.svg)](https://github.com/eigenein/blitz-dashboard/actions)
![Tag](https://img.shields.io/github/v/tag/eigenein/blitz-dashboard)

## Blitz Dashboard

![Screenshot](screenshot.png)

## Installation

Grab a binary for your Raspberry Pi from [releases](https://github.com/eigenein/blitz-dashboard/releases), or:

```shell
cargo install --git 'https://github.com/eigenein/blitz-dashboard.git' --branch main --locked
```

## Setting up

### Prerequisites

- PostgreSQL ≥ `13.4`
- Redis ≥ `6.0.15`

These are the versions I'm running with. Lower versions may work, but I haven't tested them.

### Overview

Blitz Dashboard consists of a single executable `blitz-dashboard`, which serves multiple sub-commands:

- Web application
- Account crawler
- Win rate prediction model trainer

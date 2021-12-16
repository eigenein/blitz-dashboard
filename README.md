[![Last commit](https://img.shields.io/github/last-commit/eigenein/blitz-dashboard?logo=github)](https://github.com/eigenein/blitz-dashboard/commits/master)
[![Build status](https://github.com/eigenein/blitz-dashboard/actions/workflows/check.yml/badge.svg)](https://github.com/eigenein/blitz-dashboard/actions)
![Tag](https://img.shields.io/github/v/tag/eigenein/blitz-dashboard)

## Blitz Dashboard

![Screenshot](screenshot.png)

## Installation

Grab a binary for Raspberry Pi 4 from the [releases](https://github.com/eigenein/blitz-dashboard/releases), or:

```shell
cargo install --git 'https://github.com/eigenein/blitz-dashboard.git' --branch main --locked
```

## Setting up

### Prerequisites

- PostgreSQL ≥ `13.4`
- Redis ≥ `6.2.6`

These are the versions I'm running with. Lower versions may work, but I haven't tested them.

### Overview

Blitz Dashboard consists of a single executable `blitz-dashboard`, which serves multiple sub-commands:

- Web application
- Account crawler: service and the one-off tool
- Win rate prediction model trainer
- Tankopedia importer

### «Cold» start

In order to run the crawler, you'd need to fill in the database with some accounts. The web application automatically stores all viewed accounts in the database. But you can also scan all the account ID space and import existing accounts with `blitz-dashboard crawl-accounts`.

For example:

```shell
blitz-dashboard -v crawl-accounts --initialize-schema -d postgres://pi@localhost/yastatist -a <application-ID> --start-id 1 --end-id 150000000
```

This is a **very** slow process. On average, you'll import around 1M accounts per day.

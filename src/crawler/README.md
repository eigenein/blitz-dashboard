## Example configuration

```unit file (systemd)
[Unit]
Description = Blitz Dashboard Crawler
BindsTo = network-online.target postgresql.service redis.service
After = network-online.target postgresql.service redis.service

[Service]
ExecStart = /home/pi/bin/blitz-dashboard \
    --sentry-dsn=<Sentry-DSN> \
    -v \
    crawl \
    -a=<application-ID> \
    -d=postgres://user@host/database \
    --auto-min-offset \
    --n-buffered-batches=4 \
    --n-buffered-accounts=15 \
    --log-interval=60s \
    --stream-duration=5d \
    --redis-uri=redis+unix:///var/run/redis/redis-server.sock
WorkingDirectory = /home/pi
StandardOutput = journal
StandardError = journal
Restart = always
RestartSec = 5
User = pi

[Install]
WantedBy = multi-user.target
```

## Tuning

Wargaming.net API is limited at 20 requests per second for a server-side application. For the optimal performance try and utilise 19-20 RPS for the crawler service. There's a few parameters to tune:

### `--n-buffered-batches`

Defines a number of buffered batches of accounts – these are [`account/info`](https://developers.wargaming.net/reference/all/wotb/account/info/) calls with up to 100 accounts used to check their last battle timestamps.

The more – the better. I recommend try and increase this setting one by one until you start getting `REQUEST_LIMIT_EXCEEDED`, and then use the last successful value.

### `--n-buffered-accounts`

For those accounts which last battle timestamp has changed, the crawler does a couple of more calls: [`tanks/stats`](https://developers.wargaming.net/reference/all/wotb/tanks/stats/) and [`tanks/achievements`](https://developers.wargaming.net/reference/all/wotb/tanks/achievements/). This option defines for how many accounts these calls get buffered. The more – the better, unless you start getting `REQUEST_LIMIT_EXCEEDED`.

### `--throttling-period`

Minimal period between the API calls, used to prevent the `REQUEST_LIMIT_EXCEEDED` errors. For server-side apps set this to `50ms`, for standalone apps – `100ms`.

This setting is rather **unstable**. It may cause `REQUEST_LIMIT_EXCEEDED`, or lower the RPS, or make the process stuck forever.

## «Cold» start

In order to run the crawler, you'd need to fill in the database with some accounts. The web application automatically stores all viewed accounts in the database. But you can also scan all the account ID space and import existing accounts with `blitz-dashboard crawl-accounts`.

For example:

```shell
blitz-dashboard -v crawl-accounts --initialize-schema -d postgres://pi@localhost/yastatist -a <application-ID> --start-id 1 --end-id 150000000
```

This is a **very** slow process. On average, you'll be importing around 1M accounts per day.

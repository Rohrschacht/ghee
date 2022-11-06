# ghee

## About

ghee creates and manages btrfs snapshots for backup purposes.

ghee backup jobs are specified in a config file written in either yaml, json or toml.
For each job, you define which subvolume should be snapshotted, where to store the snapshot and how long to keep it.

## Configuration

ghee will look for its configuration file at `/etc/ghee/ghee.yaml` by default. A custom configuration file can be set
using the `-c` or `--config` flag. json and toml files are also supported and examples can be found in this repo.

Let's examine the configuration at hand of the example yaml config.
For a complete example of this configuration, refer to `example-config.yaml`.

### Backup `/home`

Backs up the `/home` subvolume to `/mnt/btrfs/@/gheesnaps`. Keep at least the newest 10 snapshots.
Also keep one snapshot for each of the last 10 hours and one for each of the last 14 days.
This job will be executed when either the `volumes` or the `home` group is specified, or when no explicit group is
specified.

```yaml
- subvolume: /home # path to the subvolume that shall be backed up
  target: /mnt/btrfs/@/gheesnaps # destination for the new snapshot
  groups: # OPTIONAL: a set of groups for this job
    - volumes
    - home
  preserve: # configures how long the snapshots will be kept
    retention: 10h 14d # OPTIONAL: for the last 10 hours, keep an hourly snapshot; for the last 14 days, keep one per day.
    min: 10 # keep at least 10 snapshots, no matter how old
```

### Backup `/etc`

Backs up `/etc` to `/mnt/btrfs/@/gheesnaps`. Always keeps all snapshots from the last 5 days.
Also keep one snapshot for each of the last 48 hours ond one for each of the last 14 days.
This job will be executed when either the `volumes` or the `etc` group is specified, or when no explicit group is
specified.

```yaml
- subvolume: /etc
  target: /mnt/btrfs/@/gheesnaps
  groups:
    - volumes
    - etc
  preserve:
    retention: 48h 14d # OPTIONAL: for the last 48 hours, keep an hourly snapshot; for the last 14 days, keep one per day.
    min: 5d # keep at least all snapshots that were taken in the last 5 days
```

### Backup `/var/lib/postgres`

Backs up `/var/lib/postgres` to `/mnt/btrfs/@/gheesnaps`. Do not enforce a minimum number of snapshots to keep.
Keep 48 hourly, 14 daily, 4 weekly, 6 monthly and 2 yearly snapshots according to retention setting.
This job will be executed when the `database` group is specified, or when no explicit group is specified.

```yaml
- subvolume: /var/lib/postgres
  target: /mnt/btrfs/@/gheesnaps
  groups:
    - database
  preserve:
    retention: 48h 14d 4w 6m 2y # OPTIONAL: keep hourly snapshots for 48 hours, dailies for 14 days, weeklies for 4 weeks, monthlies for 6 months and yearlies for 2 years
    min: 0 # do not enforce a minimum number of snapshots to keep, only abide by retention setting
```

### Backup `/var/lib/mongodb`

Backs up `/var/lib/mongodb` to `/mnt/btrfs/@/gheesnaps`. Always keep all snapshots.
Since this job has no group set, it will only be executed when no group is specified explicitly.

```yaml
- subvolume: /var/lib/mongodb
  target: /mnt/btrfs/@/gheesnaps
  preserve:
    min: all # never delete any snapshots for this job
```

In addition to the `all` keyword, there is also the `latest` keyword recognized for the min preserve setting.
It means that the latest snapshot will always be kept.

## Execution of backup jobs

ghee operates in one of three modes: `run`, `dryrun` or `prune`.

`$ ghee dryrun` gathers all intents for creating, keeping and deleting snapshots and prints them out in a table.
However, it does not execute any of them. For each of the defined (or selected) jobs, ghee intents to create a new
snapshot when it is run. For all snapshots in the target location, it is decided whether to keep or delete them based on
the preserve setting.

`$ ghee run` does the same as dryrun, but also executed on the gathered intent.

`$ ghee prune` does not create new snapshots, only removes ones according to the preserve setting.

If you wish to only operate on jobs belonging to a group, specify that group after the subcommand:

```
$ ghee run home
```

A dry run can be executed for any operation (for testing what `prune` would do) by adding the `-n` or `--dryrun` flag.

The rest of the commandline interface is explained by `ghee help`:

```
Automated btrfs snapshots

Usage: ghee [OPTIONS] <COMMAND>

Commands:
  run     Runs the configured jobs, creates and prunes snapshots
  dryrun  Prints the actions that would be taken
  prune   Prunes snapshots
  help    Print this message or the help of the given subcommand(s)

Options:
  -c, --config <CONFIG>  [default: /etc/ghee/ghee.yaml]
  -n, --dryrun           Dry run, don't perform any actions
  -v, --verbose...       More output per occurrence
  -q, --quiet...         Less output per occurrence
  -h, --help             Print help information
```

## Automation

ghee is intended to be run periodically by an external service such as cron or systemd timers.

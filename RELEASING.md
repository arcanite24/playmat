# Releasing Playmat

Playmat ships from GitHub. Every master commit becomes images that the staging server (Multivac) follows. Every published release becomes images and desktop installers, and after an approval the release goes live on production (playmat.deimos.studio).

```
pull request ──► master ──► Images: playmat-server / playmat-web :sha-<7>, :master ──► Multivac (pull-images.ps1, hourly)

GitHub release vX.Y.Z ──► Images :vX.Y.Z, :latest ─┐
                       └► Desktop installers         ├─► approval (environment "production") ──► deploy on the host
                                                      │     pull · snapshot · drain · restart · health gate (rollback)
                                                      └─► smoke test from outside (rollback on failure) ──► summary
```

## Cut a release

1. Check that master is green and that Multivac, which runs master's newest images, looks right.
2. Choose the version (semver): `vX.Y.Z`. There is no version file to bump: the tag is the version. The web client shows it, the images carry it, and the desktop installers are named after it. A pre-release (`v1.3.0-rc.1`, marked as a pre-release) builds images and installers but doesn't deploy.
3. Label the merged pull requests so the notes sort them: `enhancement` (Features), `bug` (Fixes), `ops` (Ops; the labeler adds it for `ops/` and workflow changes), `skip-changelog` to leave one out. Everything else goes under Other.
4. Create the release on GitHub, from master, with generated notes:

   ```bash
   gh release create v1.3.0 --target master --generate-notes --title "Playmat 1.3.0"
   ```

   Or use Releases, Draft a new release, choose a new tag `v1.3.0` on master, Generate release notes, edit, and Publish. Players read the notes in the app ("What's new"), so write the changes for them.

   Create the release itself, not just a tag. When only a tag is pushed, the desktop workflow creates the release with the workflow's own token, and GitHub doesn't start other workflows from that event. In that case, build and deploy by hand: Actions, Images, Run workflow with the tag, then Actions, Deploy, Run workflow with the tag.

## What the pipeline does

1. **Images** (`.github/workflows/images.yml`, called by the deploy workflow). It builds `ops/web/Dockerfile.server` and `Dockerfile.web` on native amd64 and arm64 runners and pushes them by digest. Then it tags the multi-arch images `ghcr.io/arcanite24/playmat-server` and `playmat-web` as `vX.Y.Z`, `latest` (not for pre-releases) and `sha-<7>`. The web image carries the version and the release's notes as `/release-notes.json`. The server image carries the compose file the host runs. The server's full Maven build takes most of the time (tens of minutes).
2. **Desktop installers** (`desktop.yml`, on the tag). It attaches the installers to the release and adds the download notes below the generated changes.
3. **Approval.** The deploy job waits on the `production` environment. Open the run (Actions, Deploy), choose Review deployments, then Approve. Reject it, or let it time out, and nothing changes.
4. **Deploy on the host.** The job connects over SSH as `deploy` and runs `deploy vX.Y.Z`. That key can only run `ops/web/deploy/playmat-deploy`, which:
   - pulls both images and pins their digests,
   - snapshots the user databases (into `/var/backups/playmat`, keeping the newest 5),
   - drains running games (maintenance is announced, and it waits up to 15 minutes),
   - restarts on the new images,
   - and waits up to 5 minutes for `xmage`, `web` and `https://playmat.deimos.studio/` to be healthy.

   If they aren't, it rolls back to the previous digests by itself, and the job fails. Either way the staff Discord channel hears about it.
5. **Smoke test.** From the runner, `ops/web/deploy/smoke.mjs` checks three things: the page answers 200, the page is the app shell at the new version, and `serverInfo` answers over `wss://…/ws`. If any check fails, the job runs `rollback` on the host and fails.
6. **Summary.** The run's summary shows the tag, image digests, previous tag, smoke result and the host's output.
7. **Players.** The first page of the new version shows "What's new in vX.Y.Z" once. About and the Settings footer show the version.

## Redeploy or roll back

Use Actions, Deploy, Run workflow, and give it a tag. It deploys that tag's existing images without building them. The tag can be:

- `v1.2.2`: an older release (a rollback), or the current one again (a redeploy).
- `sha-abc1234` or `master`: a master build. This is for emergencies or for trying something on production.

Manual runs need the same approval. They go through the same drain, health gate and smoke test.

```bash
gh workflow run deploy.yml -f tag=v1.2.2
gh run watch
```

The host also keeps the previous version. `rollback` swaps back to it at once, without draining (see the incident commands below).

## First-time setup

Do these once, in order.

1. **Merge to master.** The first Images run on master creates the two GHCR packages. Wait for it to finish.
2. **Make the packages public.** Go to github.com/arcanite24?tab=packages, open `playmat-server`, then Package settings, Change visibility, Public. Do the same for `playmat-web`. Pulls then need no login on the host or on Multivac. To keep them private instead, set `GHCR_USER` and `GHCR_TOKEN` (a token with `read:packages`) in `/etc/playmat-deploy.conf`.
3. **Create a deploy key** on your machine, used only for this:

   ```bash
   ssh-keygen -t ed25519 -N '' -C github-actions-deploy@playmat -f playmat-deploy-key
   ```

4. **Set up the host**, as root, in the checkout that runs the stack today (`/root/xmage-server`):

   ```bash
   cd /root/xmage-server && git pull --ff-only
   ops/web/deploy/setup-deploy-user.sh --key "ssh-ed25519 AAAA... github-actions-deploy@playmat"   # the .pub line
   sudo -u deploy playmat-deploy status
   ```

   The script does the following:
   - It creates the `deploy` user (no password, docker group).
   - It installs `playmat-deploy` and `playmat-compose`, and writes `/etc/playmat-deploy.conf`. It keeps the compose project the stack already runs as (`web`), so the volumes stay the same.
   - It copies `ops/web/.env` to `/srv/playmat/.env`, where settings live from now on.
   - It records the images built on the host as the current version (`local`), which is what a first rollback returns to.

   It prints the host key line for the next step. Verify that line against `ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub` on the host, not against a scan over the network. From now on, run the stack with `playmat-compose`, not with compose from the checkout.
5. **Create the environment.** In the repository, go to Settings, Environments, New environment, and name it `production`.
   - Required reviewers: yourself. Leave "Prevent self-review" off, because you start and approve your own deploys.
   - Deployment branches and tags: Selected. Add the branch `master` and the tag pattern `v*`. Release runs use the tag ref, and manual runs use master.
   - Environment secrets:
     - `DEPLOY_HOST`: `46.224.105.107` (or `playmat.deimos.studio`).
     - `DEPLOY_SSH_KEY`: the whole private key file `playmat-deploy-key`.
     - `DEPLOY_KNOWN_HOSTS`: the line the setup printed, starting with the same name or address as `DEPLOY_HOST`, for example `46.224.105.107 ssh-ed25519 AAAA...`.
   - Environment variables:
     - `DEPLOY_URL`: `https://playmat.deimos.studio`.
     - `DEPLOY_SSH_PORT`: only if sshd isn't on 22.
   - Repository variable `APP_NAME` (optional): the product name built into the web image, `Playmat` when unset.
6. **Labels.** If the `ops` label doesn't exist yet, create it (`gh label create ops`). The labeler adds it from then on.
7. **First deploy.** In Actions, Deploy, Run workflow, give it the `sha-<7>` of the merged master and approve it. Or cut a release. The first deploy recreates the containers on the published images and the `/srv/playmat` compose kit. The volumes, and so the accounts and decks, stay.
8. **Multivac** (staging). In its checkout, `git pull`. Then create the scheduled task from `ops/web/README.md` ("Multivac follows master"). Disable any older task that builds with `up --build`.

The Hetzner firewall already allows SSH (22) from anywhere, which GitHub's runners need: their addresses change. sshd accepts keys only, and the deploy key can run nothing but `playmat-deploy`.

## Incident quick commands

On the host, as root:

```bash
sudo -u deploy playmat-deploy status            # current and previous versions, health, /healthz
sudo -u deploy playmat-deploy rollback          # back to the previous images now (no drain)
sudo -u deploy playmat-deploy deploy v1.2.2     # a specific version, with drain and health gate
tail -n 50 /var/lib/playmat-deploy/deploy.log   # what the deploys did
cat /var/lib/playmat-deploy/state               # current and previous tags and digests

playmat-compose ps
playmat-compose logs -f --tail 200 xmage
playmat-compose up -d --no-build                # after editing /srv/playmat/.env

# maintenance left on (new tables refused): call it off
playmat-compose exec -T xmage bash -c 'exec 3<>/dev/tcp/127.0.0.1/17173; printf "POST /maintenance/cancel HTTP/1.0\r\nContent-Length: 0\r\n\r\n" >&3; cat <&3'

# shut the deploy key out (GitHub can't deploy until it's written back)
: > /home/deploy/.ssh/authorized_keys
```

To restore a pre-deploy snapshot, start from `/var/backups/playmat/pre-<tag>-<time>.tar.gz`. It holds one online dump directory, named by its UTC time, as the backup service writes it. Stop the server, copy the dump into the backups volume, and restore it:

```bash
mkdir -p /tmp/restore && tar xzf /var/backups/playmat/pre-v1.3.0-20261020T180000Z.tar.gz -C /tmp/restore
ls /tmp/restore                                               # e.g. 20261020T180001Z
playmat-compose stop xmage web
playmat-compose cp /tmp/restore/20261020T180001Z backup:/backups/20261020T180001Z
playmat-compose run --rm --no-deps backup restore 20261020T180001Z
playmat-compose up -d --no-build
```

A snapshot taken while the backup service wasn't running is a copy of the database volume (`db/`). Restore it into the `web_xmage-db` volume with the server stopped.

From your machine:

```bash
gh run list --workflow deploy.yml               # recent deploys
gh workflow run deploy.yml -f tag=v1.2.2        # redeploy or roll back (needs approval)
gh workflow run images.yml -f tag=v1.3.0        # rebuild a release's images
```

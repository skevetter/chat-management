#!/usr/bin/env bash
set -e
cd "$(dirname "$0")"
CM="./target/release/chat-management"
DB="demo.db"
rm -f "$DB"

slowtype() {
  local text="$1"
  printf "\033[1;32m❯\033[0m "
  for (( i=0; i<${#text}; i++ )); do
    printf "%s" "${text:$i:1}"
    sleep 0.03
  done
  echo
  sleep 0.3
}

pause() { sleep "${1:-1.5}"; }

echo ""
echo -e "\033[1;36m═══════════════════════════════════════════════════\033[0m"
echo -e "\033[1;36m  Chat Management CLI — Full Feature Demo\033[0m"
echo -e "\033[1;36m═══════════════════════════════════════════════════\033[0m"
echo ""
pause 1

echo -e "\033[1;33m▸ Build the project\033[0m"
slowtype "cargo build --release"
cargo build --release 2>&1
pause 2

echo ""
echo -e "\033[1;33m▸ Create a channel for team discussion\033[0m"
slowtype "$CM --db $DB channel create --name demo-general --purpose 'Team discussion'"
$CM --db "$DB" channel create --name demo-general --purpose 'Team discussion'
pause 2

echo ""
echo -e "\033[1;33m▸ List all channels\033[0m"
slowtype "$CM --db $DB channel list"
$CM --db "$DB" channel list
pause 2

echo ""
echo -e "\033[1;33m▸ Post a message with an @mention\033[0m"
slowtype "$CM --db $DB post demo-general --sender alice --content 'Hello @bob! Welcome to the team.'"
$CM --db "$DB" post demo-general --sender alice --content 'Hello @bob! Welcome to the team.'
pause 1.5

echo ""
echo -e "\033[1;33m▸ Post more messages to build a conversation\033[0m"
slowtype "$CM --db $DB post demo-general --sender bob --content 'Thanks @alice! Ready to deploy.'"
$CM --db "$DB" post demo-general --sender bob --content 'Thanks @alice! Ready to deploy.'
pause 1

slowtype "$CM --db $DB post demo-general --sender charlie --content 'Deploy pipeline looks green @bob'"
$CM --db "$DB" post demo-general --sender charlie --content 'Deploy pipeline looks green @bob'
pause 1.5

echo ""
echo -e "\033[1;33m▸ Post with an idempotency key (safe to retry)\033[0m"
slowtype "$CM --db $DB post demo-general --sender alice --content 'Deploy done' --idempotency-key 'alice:deploy:2026'"
$CM --db "$DB" post demo-general --sender alice --content 'Deploy done' --idempotency-key 'alice:deploy:2026'
pause 1

echo -e "\033[1;34m  ↳ Retry the same idempotency key — no duplicate created\033[0m"
slowtype "$CM --db $DB post demo-general --sender alice --content 'Deploy done' --idempotency-key 'alice:deploy:2026'"
$CM --db "$DB" post demo-general --sender alice --content 'Deploy done' --idempotency-key 'alice:deploy:2026'
pause 2

echo ""
echo -e "\033[1;33m▸ Read messages (limit 5)\033[0m"
slowtype "$CM --db $DB read demo-general --limit 5"
$CM --db "$DB" read demo-general --limit 5
pause 2

echo ""
echo -e "\033[1;33m▸ Filter messages by sender\033[0m"
slowtype "$CM --db $DB read demo-general --sender alice"
$CM --db "$DB" read demo-general --sender alice
pause 2

echo ""
echo -e "\033[1;33m▸ Inspect channel metadata\033[0m"
slowtype "$CM --db $DB inspect demo-general"
$CM --db "$DB" inspect demo-general
pause 2

echo ""
echo -e "\033[1;33m▸ Check mentions for bob\033[0m"
slowtype "$CM --db $DB mentions --agent bob"
$CM --db "$DB" mentions --agent bob
pause 2

echo ""
echo -e "\033[1;33m▸ Show channel details\033[0m"
slowtype "$CM --db $DB channel show demo-general"
$CM --db "$DB" channel show demo-general
pause 2

echo ""
echo -e "\033[1;33m▸ Delete the channel\033[0m"
slowtype "$CM --db $DB channel delete demo-general"
$CM --db "$DB" channel delete demo-general
pause 1.5

echo ""
echo -e "\033[1;33m▸ Verify channel is gone\033[0m"
slowtype "$CM --db $DB channel list"
$CM --db "$DB" channel list
pause 1

rm -f "$DB"
echo ""
echo -e "\033[1;32m✓ Demo complete!\033[0m"
pause 1

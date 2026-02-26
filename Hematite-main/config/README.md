# Hematite Configuration Files

These JSON files are fetched by the Hematite application at runtime.

## Files

### `fix_config.json`
Contains all the active fix rules with detection and transformation logic.

**TTL:** 1 hour (re-fetches after cache expires)

### `champion_list.json`
Contains champion/subchamp mappings and known HP bar values.

**TTL:** 1 week (champions don't change often)

## Adding New Fixes

Edit `fix_config.json` and add a new entry under `fixes`. The application will pick up changes automatically when users restart or when the cache expires.

## Updating HP Bar Values

When Riot adds new characters/entities, add their HP bar style values to `healthbar_values` in `champion_list.json`.

## Schema Version

Bump the `version` field when making breaking changes to ensure clients can detect and handle them.

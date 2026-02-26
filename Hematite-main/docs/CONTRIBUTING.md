# Documentation Update Rules

## When You Make Changes:

1. **Always update TODO.md** - Move completed items, add new tasks
2. **Update CHANGELOG.md** - Document what changed
3. **Update README.md** - Modify "Current Status" section
4. **Update relevant docs** - If you change architecture, update ARCHITECTURE.md

## Before Committing:

```bash
# Run this checklist:
- [ ] Updated TODO.md with current progress
- [ ] Added entry to CHANGELOG.md
- [ ] Updated README.md status badges
- [ ] Verified all links in docs/ work
```

## For IDE Chats:

When starting a new chat with an IDE:
1. Share `docs/ARCHITECTURE.md` first
2. Share `docs/TODO.md` to show current progress
3. Share `docs/CONFIG_SCHEMA.md` if working on config
4. Always reference the specific stage you're working on

## File-Specific Guidelines

### README.md
- Update the "Current Status" section after each stage
- Keep the "Next Steps" list current
- Update timestamps when making changes

### TODO.md
- Use `[x]` for completed tasks
- Use `[/]` for in-progress tasks
- Use `[ ]` for pending tasks
- Add new tasks under the appropriate stage

### CHANGELOG.md
- Follow [Keep a Changelog](https://keepachangelog.com/) format
- Group changes under: Added, Changed, Fixed, Removed
- Include date for releases

### ARCHITECTURE.md
- Update diagrams when data flow changes
- Document new design decisions
- Keep component descriptions current

### CONFIG_SCHEMA.md
- Update as schema evolves
- Document any breaking changes
- Include examples for each field type

---

**Last Updated:** 2026-01-11

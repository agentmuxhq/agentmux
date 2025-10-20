# Upstream Sync Notes (Agent1)

- **Date:** 2025-10-08
- **Action:** Merged `upstream/main` into the fork (`a5af/waveterm`).
- **Conflict:** `frontend/app/workspace/workspace.tsx`
  - Resolved by adopting the upstream implementation (AI panel layout with `PanelGroup`), replacing the fork-only placeholder markup.
- **Follow-up:** Monitor AI panel integration in desktop build; ensure `workspace-layout-model` stays in sync with future upstream updates.

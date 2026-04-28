# Development Notes

## Pending: GUI Settings (future release)

### Cache size warning
When implementing the Settings dialog with Transmission daemon options,
`cache-size-mb` must show a danger indicator for any value > 0.

**Why:** Transmission's app-layer write cache has an unfixed bug (#5797) where
a write error (e.g. "File name too long", disk full) causes the cache to grow
indefinitely without flushing, consuming all available RAM until OOM kill.

**Recommendation:** Keep `cache-size-mb = 0`. At zero, Transmission bypasses
its own cache entirely and delegates buffering to the Linux page cache (PR #5668),
which is safer and equally performant on SSD/NVMe. The Transmission team removed
the cache feature entirely in 5.0.0-beta.1 (issue #7530).

**UI behaviour to implement:**
- Value = 0 → green indicator, label "Safe (OS cache)"
- Value > 0 → red indicator, warning text explaining the crash risk
- Suggested default to offer: 0 MB

**References:**
- https://github.com/transmission/transmission/issues/5797 (endless RAM growth bug, still open)
- https://github.com/transmission/transmission/issues/7530 (cache removal in 5.0)
- https://github.com/transmission/transmission/issues/7283 (data loss on unexpected quit)

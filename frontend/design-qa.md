# Design QA — institution clustering, ZSJ only

final result: passed

Date: 2026-10-10. Scope: improve the existing family and region maps directly, using Product Design review. No generated images, new data imports or new OTP calculations.

## Source and comparison evidence

- User source: `/var/folders/nl/r07j_tss4hs495mltd3xxrdc0000gn/T/TemporaryItems/NSIRD_screencaptureui_97fVUS/Screenshot 2026-10-10 at 10.53.49 AM.png` (map crop showing overlapping institutions and the obsolete municipality toggle).
- Matched baseline: `../output/map-panel-implementation-2026-10-10/region-closed-final.png`.
- Final evidence: `../output/map-clustering-2026-10-10/`.
- `comparison.jpg`: source and final implementation together at 1487 × 1058 CSS viewport, full-page upper viewport crops. Opened and visually reviewed.
- `map-comparison.jpg`: corresponding map crops together, also visually reviewed.
- State: region, IT, daily study, 45-minute threshold, baseline, drawer closed, all institution layers visible.
- `region-overview.png`, `family-overview.png`: final overview states.
- `region-scenario.png`: simulated field addition, inspector open.
- `family-zoomed.png`: cluster expansion and employer detail.
- `region-mobile.png`: final 390 × 844 responsive check after edge-clipping correction.
- `map-final.png`: actual browser capture with page context for handoff.
- Previous implementation report preserved in `before/design-qa.md` in this evidence directory.

## Visual and interaction assessment

| Area | Result | Evidence / decision |
| --- | --- | --- |
| Structure | Pass | Existing map, filters, drawer and statistics retained. Municipality mode removed from both dashboards; map legend explicitly says ZSJ. |
| Hierarchy | Pass | Nearby institutions use compact count markers. Geographic groups keep separate green, red and navy buttons and never mix category counts. Even a category containing one institution shows its count when adjacent to other categories. |
| Color and shape | Pass | Existing green offered schools, red other schools and navy rectangular employers retained. School/employer icons, tooltips and accessible category labels supplement color. |
| Spacing | Pass | Groups merge based on their displayed footprint; white separation makes adjacent categories legible. Original compact legend retained. |
| Zoom | Pass | Clicking a count centers and zooms to that category’s actual member coordinates. Subsequent zooms split the group until individual markers are available. Reset and manual zoom rebuild clusters. |
| Same-position data | Pass | At a sufficiently close view, colocated institutions open a compact selection list. This avoids an endless zoom loop when multiple records share identical coordinates. |
| Details | Pass | Individual school and employer markers open the existing inspector. Employer suitability and other database-backed details remain available there. |
| Keyboard | Pass | Marker controls are native buttons with descriptive labels and visible focus. Zoom is announced; the colocated picker receives focus and supports Escape with focus restoration. |
| Responsive | Pass | Reviewed at default desktop, 1487 × 1058 and 390 × 844. Counts and controls stay fully within map edges; no document horizontal overflow on mobile. |
| Data integrity | Pass | Same preprocessed snapshot and simulation matrix. Cluster numbers count school records/workplaces, not vacancies. Simulation updates school category membership. |

## Issues fixed during review

1. P1: dense city markers obscured each other. Replaced institution scatter symbols with a screen-space clustered button overlay, projecting actual coordinates through ECharts.
2. P2: the obsolete municipality switch complicated the map. Removed mode state, geometry fetch, switch and unused props; ZSJ is the only map layer.
3. P2: a new explanatory legend line made the legend unnecessarily tall. Moved the count explanation to the existing attribution line.
4. P2: mixed geographic groups with one institution in a category did not visibly display its count. They now display 1 consistently.
5. P2: mobile edge clusters could be clipped. Clamp the displayed footprint and recheck collisions after each merge; preserve actual coordinates for zoom. Only locations in the viewport contribute to displayed clusters.

No unresolved P0/P1/P2 findings in the reviewed scope.

## Functional evidence

- Region overview: 3 green schools + 31 red schools + 37 employer workplaces. Each record counted once.
- Region cluster sequence: red 20 → 12 → 8 → individual markers and pairs. A colocated pair opened the school picker; selecting TRIVIS opened the correct inspector and synchronized the simulation selector.
- Employer layer off/on removes/restores only employer counts.
- Adding IT at SLŠ Žlutice: 4 green + 30 red schools, 37 workplaces; existing coverage result remains 61% → 63%, +350 children.
- Family overview: 3 offered schools + 37 workplaces, no unoffered schools.
- Family employer cluster split into smaller groups and individual workplaces; selecting WITTE opened the correct detail, including 8 relevant positions (2 best-matched, 6 suitable).
- Mobile cluster click changed a red group of 22 into smaller groups and visible individual markers. Every displayed group stayed within the map boundary.
- No municipality toggle in either dashboard. Existing text-based list remains available.
- Browser warning/error logs: none in checked sessions.

## Automated validation

- `npm run lint`: passed.
- `npm test`: 13 passed, including 5 clustering tests: separate category counts, deterministic grouping, zoom separation, no lost/duplicated members or overlapping dense groups, bounded zoom, and mobile edge constraints preserving geographic coordinates.
- `npm run build`: passed, including TypeScript and static route generation.
- Existing non-failing Node test module-type warning remains; no unrelated package-mode change.

## Deliberate behavior

Clusters represent nearby locations at the current zoom, so counts regroup during pan/zoom and viewport changes. All markers sharing identical coordinates cannot be geographically separated; the selection list handles that case. The existing mock acceptance-rate and journey-leg labels remain unchanged.

Local preview remains available at http://localhost:3000/kraj and http://localhost:3000/rodiny.

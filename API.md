# Global Roster Manager: Backend API Requirements

This is everything the frontend needs from the backend today: the usher-facing lookup site, the coordinator and leader view, and the admin dashboard. It describes the data, every endpoint, and the rules the server must enforce. The frontend currently runs on demo data in the browser (`src/store/useStore.ts`, `src/data/mock.ts`). Each store action listed in the tables below maps to one endpoint here, so the frontend can switch over one call at a time.

Version 2 · 8 November 2026

**What changed since version 1 (5 November):**

* **New floor plan.** 83 sections, aisles 1–9, aisle doors D1–D9, two-way doors X3–X5, workers' doors X1–X2 and access doors M1–M3 (§3). 312 usher seats.
* **Sessions.** A programme such as The General Assembly 2026 runs over several days with a morning and an evening session. Each session is its own event with its own roster, reporting time and start time (§2.2). The lookup now returns every session of the programme (§4.2).
* **Duties.** Every position has a duty: seat, crowd control, greeter, door, lead, protocol or support (§2.4).
* **"I can't make it."** An usher can say they won't be at a session. Their place is left empty and flagged for cover (§4.6, §9a).
* **Coordinators and leaders.** Section coordinators sign in to see their sections and ushers in each session; leaders see their coordinators too (§2.1, §5a, §8a).
* **Copy a roster** from one session to another (§9).

---

## Contents

1. [Conventions](#1-conventions)
2. [Data models](#2-data-models)
3. [The floor plan and usher spots](#3-the-floor-plan-and-usher-spots)
4. [Public endpoints (ushers)](#4-public-endpoints-ushers)
5. [Admin: authentication](#5-admin-authentication)
    * [5a. Team: coordinators and leaders](#5a-team-coordinators-and-leaders)
6. [Admin: events](#6-admin-events)
7. [Admin: zones and positions](#7-admin-zones-and-positions)
8. [Admin: members](#8-admin-members)
    * [8a. Admin: team roles](#8a-admin-team-roles)
9. [Admin: assignments](#9-admin-assignments)
    * [9a. Admin: absences and cover](#9a-admin-absences-and-cover)
10. [Admin: venue map](#10-admin-venue-map)
11. [Admin: dashboard, search, activity, notifications](#11-admin-dashboard-search-activity-notifications)
12. [Admin: import](#12-admin-import)
13. [Admin: reports and exports](#13-admin-reports-and-exports)
14. [Admin: settings](#14-admin-settings)
15. [Business rules the server must enforce](#15-business-rules-the-server-must-enforce)
    * [Open questions](#15a-open-questions-for-the-backend-developer)
16. [Endpoint index](#16-endpoint-index)

---

## 1. Conventions

| Item | Rule |
|---|---|
| Base URL | `/api/v1` |
| Format | JSON in and out (`Content-Type: application/json`), except file upload (multipart) and exports (file download). |
| Field names | `camelCase`. |
| IDs | Opaque strings (UUID or similar). The frontend never parses them. |
| Dates | `date` fields are ISO dates (`"2026-11-14"`). Times of day are `"HH:mm"` 24-hour strings (`"15:30"`). Timestamps are ISO 8601 UTC (`"2026-11-05T14:01:42Z"`). |
| Phone numbers | Stored and returned **normalised** as 11 Nigerian digits, `08031234567`. Accept input with spaces, dashes, `+234` or `234` prefixes and normalise it (see §15.1). |
| Auth | Admin endpoints need `Authorization: Bearer <accessToken>`. Public endpoints need no token. |
| Pagination | List endpoints take `page` (from 1) and `pageSize` (default 25, max 200). They return `{ "items": [...], "page": 1, "pageSize": 25, "total": 1284 }`. |
| Sorting | `sort=<field>` and `dir=asc|desc`. |
| Errors | A non-2xx status with `{ "error": { "code": "SPOT_TAKEN", "message": "Human readable text", "fields": { "phone": "Already used" } } }`. `fields` appears only for validation errors. |
| Status codes | `200` OK, `201` created, `204` no body, `400` validation, `401` not signed in, `403` not allowed, `404` not found, `409` conflict (duplicate phone, spot taken and `replace=false`, etc.), `422` business-rule failure, `429` rate limited. |
| CORS | Allow the frontend origin(s). The app is a static site (Vite build). |

---

## 2. Data models

These match `src/data/types.ts`. Optional fields are marked `?`.

### 2.1 Member (an usher in the department)

```json
{
  "id": "m_123",
  "firstName": "Daniel",
  "lastName": "Adeyemi",
  "phone": "08031234567",
  "email?": "daniel.adeyemi@mail.com",
  "memberId?": "KGU-1001",
  "gender?": "Male | Female",
  "unit?": "Team A",
  "status": "active | inactive",
  "joinedYear?": 2018,
  "role?": "usher | coordinator | leader",
  "leaderId?": "m_12",
  "coordinates?": ["Section 40", "Section 41", "Section 42", "Section 43"],
  "createdAt": "…",
  "updatedAt": "…"
}
```

`phone` is unique across members. `memberId` is unique when present.

* `role` defaults to `usher`.
* A **coordinator** has `coordinates`, the zones they oversee, given by zone **name** ("Section 41", "Aisle 5", "Door X3"). Names are used rather than zone ids because every session has its own copy of each zone, and a coordinator looks after the same sections in every session. `leaderId` is the leader they report to.
* A **leader** has no zones of their own. They oversee every coordinator whose `leaderId` points at them, and through them those coordinators' zones.
* If you prefer, store `coordinates` as a join table (`coordinatorId`, `zoneName`). The API shape stays the same.

### 2.2 Event (one session of a ministry programme, with its own roster)

```json
{
  "id": "ev_tga26_d1am",
  "name": "The General Assembly 2026",
  "code": "TGA 2026",
  "series?": "TGA 2026",
  "day?": 1,
  "session?": "morning | evening",
  "theme?": "More Than Conquerors (Romans 8:37)",
  "description?": "…",
  "date": "2026-11-18",
  "venue": "Koinonia Global Arena, Koinonia Drive, off Olusegun Obasanjo Way, Jabi District, Abuja",
  "reportingTime": "07:30",
  "briefingTime?": "08:00",
  "startTime?": "08:45",
  "dressCode?": "Departmental Uniform",
  "status": "draft | active | completed | archived",
  "rosterStatus": "draft | published",
  "publishedAt?": "…",
  "instructions?": "Shown to ushers on their assignment page",
  "createdAt": "…",
  "updatedAt": "…"
}
```

* **A session is an event.** TGA 2026 (18–22 November) is 10 events: Day 1 morning, Day 1 evening, … Day 5 evening. Each has its own zones, positions, roster, publish state, reporting time and start time. This keeps every other endpoint unchanged: anything that takes an `eventId` works on one session.
* `series` groups the sessions of one programme. Events with the same `series` are listed together, ordered by `date`, then `session` (morning first). A one-off service has no `series`.
* `reportingTime` differs by session. The demo uses 07:30 report, 08:00 briefing and 08:45 start for mornings, and 15:45 report, 16:15 briefing and 17:00 start for evenings. The 08:45 and 17:00 start times come from the flyer; the reporting times are placeholders for the department to set.
* `startTime` is when the service starts, shown to ushers beside the reporting time.

### 2.3 Zone (where a team serves: an aisle, a section, a door or an area)

```json
{
  "id": "z_1",
  "eventId": "ev_tga26",
  "name": "Section 41",
  "kind": "aisle | section | door | area",
  "order": 47,
  "mapRef?": "S41"
}
```

`mapRef` ties the zone to the floor plan (§3). Zones without one, such as the Car Park, are listed beside the map as "Outside the hall".

### 2.4 Position (a role inside a zone)

```json
{
  "id": "p_1",
  "zoneId": "z_1",
  "name": "Front row, left end",
  "order": 0,
  "required?": 1,
  "isLeadRole?": false,
  "duty?": "seat | crowd | greeter | door | lead | protocol | support",
  "spot?": "FL | FR | BL | BR"
}
```

* `required` is the target headcount, used for coverage.
* `spot` marks a position as one of the four **usher spots** of a section (§3.2). A spot holds exactly one usher, so its `required` is always 1.
* `isLeadRole` marks roles such as "Aisle Lead".
* `duty` says what kind of work the position is. The usher's page shows it as a label, and admins filter the roster by it.

| `duty` | Label | Typical positions |
|---|---|---|
| `seat` | Seat usher | The four usher seats in a section, Overflow Seating |
| `crowd` | Crowd control | Crowd Control and Floor Marshal in each aisle, at the two-way doors and the Main Gate, Walkway Control, Traffic Direction |
| `greeter` | Greeter | Greeters at the aisle doors, the two-way doors and the Main Gate |
| `door` | Door usher | Door Usher at every door |
| `lead` | Team lead | Aisle Lead |
| `protocol` | Protocol | Altar, Apostle's chairs, protocol row, Lady Sandra's and ministers' sections, ministers' escort |
| `support` | Support | Technical, media, stage, registration, first aid |

If `duty` is missing, treat spot positions as `seat`, lead roles as `lead`, and anything else as `support`.

### 2.5 Assignment (a member placed in a position for an event)

```json
{
  "id": "as_1",
  "memberId": "m_123",
  "eventId": "ev_tga26",
  "zoneId": "z_1",
  "positionId": "p_1",
  "isTeamLead": false,
  "status": "assigned | confirmed | unavailable",
  "unavailableReason?": "Travelling for work",
  "unavailableAt?": "…",
  "updatedAt": "…"
}
```

`unavailable` means the usher said they can't make this session (§4.6). The assignment is **kept** so admins can see who dropped out, but it no longer counts as filling the position: coverage, spot occupancy, team lists and exports all leave it out. An unavailable assignment is never a team lead.

### 2.6 Activity (audit feed)

```json
{ "id": "ac_1", "actor": "Grace", "text": "assigned 12 ushers to Aisle 4", "at": "…", "kind": "assign | event | import | publish | member | position | absence | team" }
```

### 2.7 Notification (admin bell)

```json
{ "id": "n_1", "title": "Unassigned members remaining", "body": "364 members have no position for TGA 2026.", "at": "…", "tone": "info | success | warn", "read": false }
```

### 2.8 Settings

```json
{
  "currentEventId": "ev_tga26",
  "contactName": "Department Secretariat",
  "contactPhone": "08020001122",
  "showTeammatePhones": true,
  "allowSelfConfirm": true
}
```

### 2.9 Admin user

```json
{ "id": "u_1", "name": "Grace", "email": "grace@koinonia.org", "role": "owner | admin | editor" }
```

`editor` can assign and edit members but not publish, delete events or change settings. If you only need one role for now, ship `admin` and keep the field.

### 2.10 Computed: ZoneStat

Returned by several endpoints. Coverage is `min(100, round(assigned / required × 100))`. When `required` is 0, coverage is 100 if anyone is assigned, otherwise 0.

```json
{
  "zoneId": "z_1",
  "assigned": 3,
  "required": 4,
  "coverage": 75,
  "lead?": { "id": "m_2", "firstName": "John", "lastName": "Adewale", "phone": "08020001122" }
}
```

### 2.11 Computed: EventStats

```json
{
  "totalMembers": 1284,
  "assigned": 920,
  "unassigned": 364,
  "confirmed": 732,
  "positions": 377,
  "zones": 119,
  "required": 1001,
  "coverage": 71.7,
  "unavailable": 7,
  "needsCover": 5
}
```

`assigned` counts members with an assignment in the event. `unassigned = totalMembers − assigned`. `coverage` here is `assigned / totalMembers × 100`, rounded to one decimal place. This is the figure on the dashboard cards.

`assigned`, `confirmed` and coverage ignore unavailable assignments. `unavailable` counts assignments with that status. `needsCover` counts the ones whose position has not been filled again: for a seat, nobody else holds it; for other positions, the position is below its `required` headcount.

---

## 3. The floor plan and usher spots

The drawing itself lives in the frontend (`src/data/venue.ts`). The backend only stores **references** to it, so it never needs coordinates. The hall is 105 m by 90 m, split into three 30 m naves: Nave C (the two-way doors side), Nave B (the middle, facing the altar) and Nave A (the workers' doors side).

### 3.1 Plan references (`Zone.mapRef`)

| Ref | Meaning |
|---|---|
| `S1` … `S83` | Seating sections, 132 seats each (11 rows of 12). Seven columns. The column nearest the altar holds 1–11, top to bottom; the next column back holds 12–23, then 24–35, and so on to 72–83 at the back wall. |
| `A1` … `A9` | Aisles 1–9, running across the hall between section rows, each from its door on the back wall. A4–A6 are the Nave B central lane. A5 and A6 stop at the technical and media block (sections 17 and 18). |
| `D1` … `D9` | Aisle doors on the back wall, one per aisle. **D2, D5 and D8 stay closed during service.** |
| `X3`, `X4`, `X5` | Two-way doors (entrance and exit) on the Nave C wall. The public comes in here. |
| `X1`, `X2` | Workers-only entrance and exit, on the Nave A wall. |
| `M1` | Access door for ministers and the Rotunda, onto the covered walkway. |
| `M2` | Worship team access door. |
| `M3` | Access door beside the worship team, near Section 1. |
| `altar` | Altar and pulpit platform (30 m, Nave B). |
| `worship` | Worship team area. |
| `screen` | Crane LED screen, in the 1 m strip behind the stage. |
| `tech` | Technical space (stage, beside the altar). |
| `walkway` | Covered, screened walkway from M1 to the stage. |
| `reserve` | Stage reserve area. |
| `apostle` | The nine special chairs for the Apostle and guest ministers. |
| `protocol` | Protocol row (front row of Section 6). |

Fixed facts about sections, which the frontend already knows (listed for reference):

* International guests: 8, 9, 19, 20, 21, 31, 32, 33.
* Lady Sandra's section: 5. Protocols: 6. Ministers, HODs and sons of the ministry: 7. Technical: 17. Media and videography: 18.
* The top and bottom rows of each column (1, 11, 12, 23, 24, 35 … 72, 83) are the outer sections by the exit doors, 11 × 11 and drawn dashed.
* The two tent lines (below rows 3 and 7) are pillar lines, not aisles, so they have no `mapRef`.

### 3.2 Usher seats

Every section **except 5, 6, 7, 17 and 18** has four usher seats, at the first and last row, both ends. That is 78 sections × 4 = **312 seats**. Each seat is a `Position` in that section's zone, with `spot` set and `duty: "seat"`:

| `spot` | Name | Where |
|---|---|---|
| `FL` | Front row, left end | Row nearest the altar, left as you face the altar |
| `FR` | Front row, right end | Row nearest the altar, right end |
| `BL` | Back row, left end | Back row, left end |
| `BR` | Back row, right end | Back row, right end |

The frontend draws a dot for each seat and fills it from the roster. A filled dot means someone is assigned to that position, and the usher's own dot is highlighted on their page. A seat whose usher said they can't make it is drawn as **needs cover** until someone else takes it. **The map is populated from the assignment endpoints; nothing about who stands where is hard-coded.**

Sections 5, 6, 7, 17 and 18 have no seats. They get ordinary positions instead (§3.3).

### 3.3 Default venue template

When a session is created without a source to copy from, the server should create this structure (it is what `createSeed()` in `src/data/mock.ts` builds). Each line is `Position name (required, duty)`.

* **9 aisle zones** `Aisle 1`–`Aisle 9` (`A1`–`A9`): Aisle Lead (1, lead, `isLeadRole`), Crowd Control (6, crowd), Floor Marshal (3, crowd).
* **83 section zones** `Section 1`–`Section 83` (`S1`–`S83`). Sections with seats get the four seat positions. The others:
  * Section 5: Section Usher (2, protocol)
  * Section 6: Protocol Support (3, protocol)
  * Section 7: Section Usher (2, protocol)
  * Section 17: Tech Liaison (1, support)
  * Section 18: Media Liaison (1, support)
* **19 door zones**, named `Door D1` … `Door M3`:
  * Open aisle doors (D1, D3, D4, D6, D7, D9): Door Usher (2, door), Greeter (1, greeter)
  * Closed aisle doors (D2, D5, D8): Door Usher (1, door)
  * Two-way doors (X3–X5): Greeter (4, greeter), Door Usher (3, door), Crowd Control (3, crowd)
  * Workers' doors (X1, X2): Door Usher (2, door)
  * Access doors (M1–M3): Door Usher (2, door)
* **Area zones on the plan:** Altar & Pulpit (Altar Protocol 12, protocol), Worship Team (Stage Support 6, support), Apostle's Chairs (Chair Protocol 9, protocol), Protocol Row (Protocol Support 6, protocol), Technical Space (Tech Liaison 3, support), Covered Walkway (Walkway Control 4, crowd).
* **Area zones outside the hall (no `mapRef`):** Main Gate (Greeter 20, greeter; Crowd Control 12, crowd), Registration Area (Registration Desk 24, support), Car Park (Traffic Direction 30, crowd), Rotunda (Ministers' Escort 12, protocol), Overflow (Overflow Seating 30, seat), First Aid Point (First Aid Support 8, support).

Expose the template as `GET /venue/template` (§7) so admins can rebuild a session's structure.

---

## 4. Public endpoints (ushers)

No auth. Rate-limit by IP: 20 lookups per minute is plenty. Never return other members' phone numbers unless `settings.showTeammatePhones` is true, and never return another member's email.

### 4.1 `GET /public/current-event`

Feeds the event card on the landing page: the dates, the morning and evening times, the venue and the theme.

**200**
```json
{
  "event": { "id": "ev_tga26_d1am", "name": "The General Assembly 2026", "code": "TGA 2026", "series": "TGA 2026", "day": 1, "session": "morning", "theme": "…", "date": "2026-11-18", "venue": "…", "reportingTime": "07:30", "briefingTime": "08:00", "startTime": "08:45", "dressCode": "…", "rosterStatus": "published" },
  "sessions": [
    { "id": "ev_tga26_d1am", "day": 1, "session": "morning", "date": "2026-11-18", "reportingTime": "07:30", "startTime": "08:45", "rosterStatus": "published" },
    { "id": "ev_tga26_d1pm", "day": 1, "session": "evening", "date": "2026-11-18", "reportingTime": "15:45", "startTime": "17:00", "rosterStatus": "published" }
  ],
  "stats": { "assigned": 1004 }
}
```

* `event` is `settings.currentEventId`, or `null` when none is set.
* `sessions` lists every event with the same `series`, ordered by date then session. It is `[event]` for a one-off service.
* `stats.assigned` is the "Ushers serving" figure; send it only when the roster is published.

### 4.2 `POST /public/lookup`

The core usher flow: enter a phone number and see where you serve in each session.

**Body** `{ "phone": "0803 123 4567", "eventId?": "ev_…" }`. Without `eventId`, use `settings.currentEventId`. The response covers every session in that event's `series`.

**200** returns one of two results, told apart by `kind`:

```json
{ "kind": "not-found", "phone": "08099990000", "contact": { "name": "Department Secretariat", "phone": "08020009988" } }
```

```json
{
  "kind": "member",
  "member": { "id": "m_1", "firstName": "Daniel", "lastName": "Adeyemi" },
  "series": "TGA 2026",
  "sessions": [
    {
      "event": { …Event },
      "view": {
        "assignment": { "id": "as_1", "status": "confirmed", "isTeamLead": false },
        "zone": { "id": "z_s41", "name": "Section 41", "kind": "section", "mapRef": "S41" },
        "position": { "id": "p_s41_FL", "name": "Front row, left end", "spot": "FL", "duty": "seat" },
        "teamLead": { "firstName": "John", "lastName": "Adewale", "phone": "08020001122" },
        "coordinator": { "firstName": "John", "lastName": "Adewale", "phone": "08020001122" },
        "teammates": [
          { "member": { "firstName": "Samuel", "lastName": "Adeyemi", "phone?": "08030004455" }, "position": { "id": "…", "name": "Front row, right end", "spot": "FR" }, "isTeamLead": false, "isYou": false }
        ]
      }
    },
    { "event": { …Event, "rosterStatus": "draft" } },
    { "event": { …Event } }
  ],
  "others": [ …AssignmentView for this member in other published events outside the series ]
}
```

One entry per session, in order. The frontend shows a session picker and works out each session's state from the entry:

| Entry | Meaning | Shown to the usher |
|---|---|---|
| `rosterStatus: "draft"`, no `view` | Roster not published yet | "Roster not published yet". Do **not** include `view` for a draft session, even if the member is assigned. |
| Published, no `view` | Not assigned this session | "No post this session" with the contact |
| `view.assignment.status` is `assigned` or `confirmed` | Serving | Post, duty, seat on the map, team, coordinator, report and start times |
| `view.assignment.status` is `unavailable` | They said they can't make it | "Marked as away" with an undo button, and `unavailableReason` |

* `teammates` covers everyone else in the same zone in that session, lead first, then by position order. **Leave out teammates who are unavailable**, but always include the member themself (`isYou: true`). Include `phone` only if `showTeammatePhones` is on.
* `teamLead` is the zone's lead, never an unavailable one.
* `coordinator` is the member whose `coordinates` include this zone's name (§2.1), with their phone (a coordinator's number is always shown).
* `others` is the member's assignments in other published, non-archived events that are not in this series, upcoming first. Each has the shape of `view` plus `event`, without `teammates`.

### 4.3 `GET /public/events/{eventId}/map`

Fills the usher's floor plan for one session: which seats are taken. Only for **published** events.

**200**
```json
{
  "zones": [ { "id": "z_s41", "name": "Section 41", "kind": "section", "mapRef": "S41" } ],
  "spots": [ { "positionId": "p_s41_FL", "zoneId": "z_s41", "spot": "FL", "name": "Front row, left end", "taken": true, "needsCover": false } ]
}
```

Return `taken` and `needsCover` only, not names, for the public map. An unavailable assignment does not make a seat `taken`. The usher's own teammates come from the lookup response.

To save a round trip, the server may embed this object in the `found` lookup response as `current.map`.

### 4.4 `POST /public/assignments/{assignmentId}/confirm`

"Confirm attendance" button. Only allowed when `settings.allowSelfConfirm` is true.

**Body** `{ "phone": "08031234567" }`. The phone must belong to the assignment's member, so ushers can't confirm for each other.

**200** `{ "assignment": { "id": "…", "status": "confirmed" } }`. **403** if self-confirm is off or the phone doesn't match.

### 4.5 `GET /public/settings`

**200** `{ "contactName": "…", "contactPhone": "…", "showTeammatePhones": true, "allowSelfConfirm": true }`

### 4.6 `POST /public/assignments/{assignmentId}/unavailable`

The "I can't make it" button on a session. The usher's place is left empty for that session so an admin, coordinator or leader can assign someone else.

**Body** `{ "phone": "08031234567", "reason?": "Travelling for work" }`. The phone must belong to the assignment's member. `reason` is optional, up to 200 characters.

**200** `{ "assignment": { "id": "…", "status": "unavailable", "unavailableReason": "…" } }`.

The server must:

* set `status: "unavailable"`, `unavailableReason`, `unavailableAt`, and clear `isTeamLead`;
* keep the assignment row (don't delete it);
* log an Activity, kind `absence`, actor the member's name: "can't make TGA 2026 · Day 1 Morning (Section 41)";
* create a `warn` notification "Spot needs cover" for admins, and notify that zone's coordinator and their leader (§5a);
* refuse with **422** if the session has already started (its `date` and `startTime` are in the past).

**403** if the phone doesn't match. **409** if the assignment is already unavailable.

### 4.7 `POST /public/assignments/{assignmentId}/available`

Undo, for an usher who can come after all. **Body** `{ "phone" }`.

* Sets `status` back to `assigned` and clears the reason.
* For a **seat**, refuse with **409** `SPOT_COVERED` if someone else now holds that seat. The frontend then tells the usher their seat has been given to someone else and to contact their coordinator.
* For other positions, always allowed.
* Logs an Activity.

**200** `{ "assignment": { …, "status": "assigned" } }`.

---

## 5. Admin: authentication

| Method | Path | Purpose |
|---|---|---|
| `POST` | `/auth/login` | Body `{ "email", "password", "remember": true }`. **200** `{ "accessToken", "refreshToken", "expiresIn": 3600, "user": AdminUser }`. **401** on bad credentials. |
| `POST` | `/auth/refresh` | Body `{ "refreshToken" }`. **200** returns new tokens. |
| `POST` | `/auth/logout` | Revokes the refresh token. **204**. |
| `GET` | `/auth/me` | **200** `AdminUser`. Used on page load to keep the session. |
| `POST` | `/auth/forgot-password` | Body `{ "email" }`. **204** always, so the response never reveals whether an account exists. |
| `POST` | `/auth/reset-password` | Body `{ "token", "password" }`. **204**. |

`remember: true` gives a long-lived refresh token (30 days). Otherwise the session lasts until the browser closes (12 hours max).

---

## 5a. Team: coordinators and leaders

Section coordinators and leaders sign in on `/team/login` with their registered phone number. This is separate from admin sign-in: a team token can only call the endpoints in this section, and they are **read-only** (covering an absence is still done by an admin).

### Sign-in

| Method | Path | Notes |
|---|---|---|
| `POST` | `/team/auth/request-code` | Body `{ "phone" }`. If the number belongs to a member with `role` `coordinator` or `leader`, send a 6-digit one-time code by SMS or WhatsApp. **204** always, so the response never reveals who is on the team. Rate-limit to 3 codes per number per 15 minutes. |
| `POST` | `/team/auth/verify` | Body `{ "phone", "code" }`. **200** `{ "accessToken", "expiresIn": 43200, "member": { id, firstName, lastName, phone, role } }`. **401** `BAD_CODE`. **403** `NOT_TEAM` with "This number isn't set up as a coordinator or leader". |
| `POST` | `/team/auth/logout` | **204**. |

The demo signs in with the phone number alone (`teamLogin` in the store returns `ok`, `not-found` or `not-team`). **The live version must use the code step**, because the team view shows other members' phone numbers. The frontend will add the code screen when these endpoints exist.

### `GET /team/me?eventId=`

Everything on the team page for one session. Without `eventId`, use `settings.currentEventId`.

**200**
```json
{
  "me": { "id": "m_2", "firstName": "John", "lastName": "Adewale", "phone": "08020001122", "role": "coordinator", "leader?": { "id": "m_12", "firstName": "Michael", "lastName": "Danladi", "phone": "08141230008" } },
  "event": { …Event },
  "sessions": [ { "id": "…", "day": 1, "session": "morning", "date": "…", "rosterStatus": "published" } ],
  "coordinators": [
    {
      "member": { "id": "m_2", "firstName": "John", "lastName": "Adewale", "phone": "08020001122" },
      "zones": [
        {
          "zone": { "id": "z_s41", "name": "Section 41", "kind": "section", "mapRef": "S41" },
          "stats": { "assigned": 3, "required": 4, "coverage": 75 },
          "positions": [ { "id": "p_s41_FL", "name": "Front row, left end", "spot": "FL", "duty": "seat", "required": 1 } ],
          "ushers": [ { "assignmentId": "as_1", "member": { "id": "m_1", "firstName": "Daniel", "lastName": "Adeyemi", "phone": "08031234567" }, "positionId": "p_s41_FL", "status": "confirmed", "isTeamLead": false } ],
          "unavailable": [ { "assignmentId": "as_9", "member": { "firstName": "Grace", "lastName": "Emmanuel", "phone": "08050007788" }, "positionId": "p_s41_BR", "reason?": "…", "covered": false } ]
        }
      ]
    }
  ],
  "stats": { "zones": 4, "assigned": 15, "required": 16, "unavailable": 1, "needsCover": 1 }
}
```

* For a **coordinator**, `coordinators` has one entry: themself, with the zones of this session whose names are in their `coordinates`.
* For a **leader**, `coordinators` has every member whose `leaderId` is the leader, each with their zones. A leader sees all of it.
* Zones are matched by name, so the same coordinator covers "Section 41" in every session.
* While the session's roster is a draft, return the zones and positions but leave `ushers` and `unavailable` empty and set `"rosterStatus": "draft"`, the same rule as the public lookup.
* `ushers` excludes unavailable assignments; they are listed in `unavailable` with `covered` (true once someone else holds the seat, or the position is back at its headcount).
* The frontend also draws the team's zones on the plan using `GET /team/events/{eventId}/map`, which returns the same shape as the admin map (§10) limited to the team's zones (other seats are only `taken: true`, with no names).

### `GET /team/events/{eventId}/map`

As above. **403** if the caller has no zones in that session.

---

## 6. Admin: events

| Method | Path | Store action | Notes |
|---|---|---|---|
| `GET` | `/events` | list | Query: `status` (comma list), `q`, `series`. Order by date, then session (morning first). Returns all events (there are few), each with `stats: EventStats`. Used by the event selector, the Events page and the Rosters page. |
| `POST` | `/events` | `createEvent` | Body: Event fields. Optional `copyStructureFrom: eventId` copies that event's zones and positions (default: the current event). Optional `useTemplate: true` builds §3.3 instead. **201** Event. |
| `GET` | `/events/{id}` | | Event plus `stats`. |
| `PATCH` | `/events/{id}` | `updateEvent` | Partial update. |
| `POST` | `/events/{id}/duplicate` | `duplicateEvent` | Copies the event, its zones and positions (not assignments). Name gets " (Copy)", code gets " COPY", status `draft`, rosterStatus `draft`. **201** Event. |
| `POST` | `/events/{id}/archive` | `archiveEvent` | Sets status `archived`. |
| `DELETE` | `/events/{id}` | `deleteEvent` | Deletes the event with its zones, positions and assignments. Refuse (**422**) for the current event. |
| `POST` | `/events/{id}/publish` | `setRosterStatus('published')` | Sets `rosterStatus=published`, `publishedAt=now`. Ushers can now see positions. Logs activity and creates a notification. |
| `POST` | `/events/{id}/unpublish` | `setRosterStatus('draft')` | Hides positions from ushers again. |
| `PUT` | `/settings/current-event` | `setActiveEvent` | Body `{ "eventId" }`. This is the session the admin works on and the lookup starts from (the lookup returns its whole series). |
| `POST` | `/series/{series}/sessions` | | Optional helper: body `{ "days": 5, "startDate": "2026-11-18", "morning": { reportingTime, briefingTime, startTime }, "evening": {…}, "copyStructureFrom?": eventId }` creates one event per day and session. The frontend's Event form creates sessions one at a time today, so this can come later. |

---

## 7. Admin: zones and positions

The Positions page, the board and every zone dropdown use these.

| Method | Path | Store action | Notes |
|---|---|---|---|
| `GET` | `/events/{eventId}/zones` | | Query `include=positions,stats`. Returns zones in `order`, each with `positions[]` (ordered) and `stats: ZoneStat`. **This one call is enough to draw the board and the zone lists.** |
| `POST` | `/events/{eventId}/zones` | `addZone` | Body `{ "name", "kind", "mapRef?" }`. If `mapRef` is missing, derive it from the name ("Section 14" → `S14`, "Aisle 2" → `A2`, "Door D4" → `D4`). **409** if another zone in the event already has that `mapRef`. Adding a section zone that has spots should create its four spot positions automatically. |
| `PATCH` | `/zones/{zoneId}` | `renameZone` | `{ "name?", "mapRef?", "order?" }`. |
| `DELETE` | `/zones/{zoneId}` | `deleteZone` | Also deletes its positions and assignments. |
| `PUT` | `/events/{eventId}/zones/order` | | Body `{ "zoneIds": [...] }`. |
| `POST` | `/zones/{zoneId}/positions` | `addPosition` | `{ "name", "required?": 4, "isLeadRole?", "duty?", "spot?" }`. **409** if the zone already has a position with that `spot`. |
| `PATCH` | `/positions/{positionId}` | `updatePosition` | `{ "name?", "required?", "isLeadRole?", "duty?" }`. `required` must stay 1 for spot positions. |
| `DELETE` | `/positions/{positionId}` | `deletePosition` | Also deletes its assignments. |
| `PUT` | `/zones/{zoneId}/positions/order` | `reorderPositions` | Body `{ "positionIds": [...] }`. |
| `GET` | `/venue/template` | | Returns the §3.3 default structure as `{ zones: [{ name, kind, mapRef, positions: [...] }] }`. |
| `POST` | `/events/{eventId}/zones/apply-template` | | Adds any template zones and positions the event is missing. Body `{ "mode": "add-missing" }`. Never deletes. |

---

## 8. Admin: members

| Method | Path | Store action | Notes |
|---|---|---|---|
| `GET` | `/members` | | Query: `q` (name, phone, member ID), `status`, `unit`, `gender`, `eventId` plus `assignment=all|assigned|unassigned`, `sort` (`name`, `phone`, `unit`, `joinedYear`, `zone`), `dir`, `page`, `pageSize`. With `eventId`, each item also has `assignment?: { id, zoneId, zoneName, positionId, positionName, status, isTeamLead }` for that event. |
| `GET` | `/members/{id}` | | Member plus `assignments[]`, one per event (with event code, name and date, zone and position names), for the member drawer. |
| `POST` | `/members` | `addMember` | **409** `DUPLICATE_PHONE` with the existing member's id and name. |
| `PATCH` | `/members/{id}` | `updateMember` | Same duplicate check. |
| `DELETE` | `/members/{id}` | | Also removes their assignments. |
| `POST` | `/members/bulk-delete` | `deleteMembers` | `{ "memberIds": [...] }`. **200** `{ "deleted": 12 }`. |
| `GET` | `/members/check-phone?phone=` | | `{ "available": false, "member?": { id, name } }`. Used for live validation in the member form. |

`GET /members` also accepts `role=coordinator|leader`, and each item includes `role`, `leaderId` and `coordinates`.

---

## 8a. Admin: team roles

The Team page (`/admin/team`) sets up leaders and coordinators. Each call maps to the store action `setTeamRole(memberId, role, { leaderId, coordinates })`.

| Method | Path | Notes |
|---|---|---|
| `GET` | `/team` | **200** `{ "leaders": [ { member, coordinators: [ { member, coordinates: ["Section 40", …] } ] } ], "unassignedCoordinators": [ … ], "uncoveredZones": ["Section 12", "Door X3", …] }`. `uncoveredZones` lists zone names of the current session that no coordinator covers; the page warns about them. |
| `PUT` | `/members/{id}/role` | Body `{ "role": "usher | coordinator | leader", "leaderId?": "m_12", "coordinates?": ["Section 40", "Section 41"] }`. **422** if `leaderId` is not a leader. Setting `usher` clears `leaderId` and `coordinates`. Changing a leader to anything else clears `leaderId` on their coordinators (they stay coordinators). Logs an Activity, kind `team`. |
| `GET` | `/zones/names` | Distinct zone names across the current series, grouped by kind, for the coordinator dialog's zone picker. |

A zone name may belong to more than one coordinator; the usher page shows the first one (by name). If that is not wanted, return **409** `ZONE_HAS_COORDINATOR` from `PUT /members/{id}/role` instead.

---

## 9. Admin: assignments

| Method | Path | Store action | Notes |
|---|---|---|---|
| `GET` | `/events/{eventId}/assignments` | | Query: `q` (member name or phone), `zoneId`, `positionId`, `status` (`assigned`, `confirmed`, `unavailable`), `duty`, `sort` (`member`, `zone`, `position`, `status`), `dir`, `page`, `pageSize`. Each item: Assignment plus `member: { id, firstName, lastName, phone }`, `zoneName`, `positionName`, `duty`, `spot?`. Without a `status` filter, unavailable assignments are included (the table shows them as "Can't make it"). |
| `POST` | `/events/{eventId}/assignments` | `assign` | Body `{ "memberId", "zoneId", "positionId", "isTeamLead?": false, "replace?": true }`. Follows the rules in §15.2–15.4. **201** `{ "assignment", "replaced?": { "previousOfMember?": Assignment, "displacedFromSpot?": { assignment, member } } }` so the UI can say what changed. |
| `PATCH` | `/assignments/{id}` | | `{ "zoneId?", "positionId?", "isTeamLead?", "status?" }`. Same rules. |
| `DELETE` | `/assignments/{id}` | `removeAssignment` | **204**. |
| `POST` | `/events/{eventId}/assignments/bulk` | `bulkAssign` | `{ "memberIds": [...], "zoneId", "positionId" }`. **200** `{ "assigned": 10, "skipped": [{ memberId, reason }] }`. For a spot position only one member may be sent (**422** otherwise). |
| `POST` | `/events/{eventId}/assignments/bulk-remove` | `removeAssignmentsFor` | `{ "memberIds": [...] }`. **200** `{ "removed": 10 }`. |
| `POST` | `/events/{eventId}/assignments/copy` | `moveToEvent` | `{ "toEventId", "memberIds?": [...] }`. Copies those members' assignments into the other event, matched by zone and position name. Members already assigned there are overwritten. **200** `{ "copied": 52, "unmatched": [...] }`. |
| `POST` | `/events/{eventId}/roster/copy-from` | `copyRoster` | Body `{ "fromEventId" }`. "Copy from…" on the Rosters page: copies every assignment of another session into this one, matched by zone and position name (the same rules as `assignments/copy`). Unavailable assignments are **not** copied; their member is left unassigned. **200** `{ "copied": 1004, "unmatched": [...] }`. Refuse with **422** if this session's roster is published. |
| `POST` | `/assignments/{id}/confirm` | `confirmAssignment` | Admin marks someone confirmed. |

`PATCH /assignments/{id}` may also set `status: "unavailable"` with `unavailableReason` (an admin recording an absence on someone's behalf), or set it back to `assigned`. The same rules as §4.6 and §4.7 apply.

When an admin assigns someone to a seat whose current holder is unavailable, the unavailable assignment stays as it is (it is not "displaced"), and the seat counts as covered.

---

## 9a. Admin: absences and cover

| Method | Path | Notes |
|---|---|---|
| `GET` | `/events/{eventId}/absences` | Query `zoneName` (repeatable), `covered=true|false`. **200** `[ { "assignmentId", "member": { id, firstName, lastName, phone }, "zone": { id, name, mapRef }, "position": { id, name, spot?, duty }, "reason?", "at", "covered": false } ]`, newest first. Feeds the "Can't make it" panel on the Overview and the team page. |

To cover an absence, the Overview's **Cover** button opens the assign dialog with that zone and position already chosen, then calls `POST /events/{eventId}/assignments` as usual.

---

## 10. Admin: venue map

The admin map shows staffing by section (badge colour) and every usher spot (filled or open). Tapping an open spot opens the assign dialog with that section and spot already chosen. Tapping a filled one shows the section's team.

### `GET /events/{eventId}/map`

```json
{
  "zones": [
    { "id": "z_s41", "name": "Section 41", "kind": "section", "mapRef": "S41", "stats": { "assigned": 4, "required": 4, "coverage": 100 } }
  ],
  "spots": [
    {
      "positionId": "p_s41_FL",
      "zoneId": "z_s41",
      "spot": "FL",
      "name": "Front row, left end",
      "occupant": { "assignmentId": "as_1", "memberId": "m_1", "firstName": "Daniel", "lastName": "Adeyemi", "status": "confirmed" },
      "away": null
    },
    { "positionId": "p_s41_BR", "zoneId": "z_s41", "spot": "BR", "name": "Back row, right end", "occupant": null, "away": { "assignmentId": "as_9", "firstName": "Grace", "lastName": "Emmanuel", "reason?": "…" } }
  ]
}
```

* Include every zone of the event (zones without a `mapRef` too; the frontend lists them as "Outside the hall").
* `spots` has one entry per spot position, even when it is open (`occupant: null`).
* `occupant` never holds an unavailable assignment. `away` is set when the seat's last usher said they can't make it and nobody has taken it since: the map draws it as **needs cover**.
* Zone `stats` ignore unavailable assignments.
* This and `GET /events/{eventId}/zones?include=positions,stats` together are all the map needs. To fill a spot, the frontend calls `POST /events/{eventId}/assignments` with that `positionId`.

---

## 11. Admin: dashboard, search, activity, notifications

| Method | Path | Notes |
|---|---|---|
| `GET` | `/events/{eventId}/overview` | Everything on the Overview page in one call: `{ event, sessions: [the series' sessions, each with rosterStatus and stats.assigned], stats: EventStats, zoneStats: [ZoneStat + name, kind, mapRef], byDuty: { seat: { assigned, required }, crowd: {…}, greeter: {…}, door: {…}, lead: {…}, protocol: {…}, support: {…} }, absences: [latest 8 from §9a], activity: [latest 7 Activity] }`. The page also draws the compact map from `/events/{eventId}/map`. |
| `GET` | `/search?q=&eventId=` | Command palette (Ctrl/⌘ K). **200** `{ "members": [max 6, with their assignment in eventId], "zones": [max 6], "events": [max 4] }`. Match names, phone digits (partial, e.g. `0803`), member IDs, zone names ("Section 14", "Aisle 3", "X3", "D4"). |
| `GET` | `/activity?limit=60` | Newest first. Written by the server on every change listed in §15.7. |
| `GET` | `/notifications` | `{ "items": [Notification], "unread": 3 }`. |
| `POST` | `/notifications/read-all` | Marks all read. **204**. |
| `POST` | `/notifications/{id}/read` | **204**. |

Notifications the server should create on its own:

* An import completed, with counts of added, updated and skipped rows.
* Duplicate phone numbers found during an import.
* A roster was published.
* An event is 21 days away or less (send once).
* Unassigned members remain for the current event (at most once a day).
* An usher said they can't make a session ("Spot needs cover", `warn`), §4.6.

---

## 12. Admin: import

A 4-step flow on the Import page: upload, preview, validate, import.

### 12.1 `GET /imports/template?format=csv|xlsx`

Downloads a template with the columns `first_name, last_name, phone, email, gender, unit, zone, position`.

### 12.2 `POST /imports/preview` (multipart)

Fields: `file` (.csv, .xlsx or .xls, max 5 MB) and `eventId` (where zone and position columns get assigned).

Header matching is case- and punctuation-insensitive:

| Field | Accepted headers |
|---|---|
| `firstName` | first name, first, given name |
| `lastName` | last name, last, surname |
| `fullName` | name, full name (split into first and last when those columns are absent) |
| `phone` | phone, phone number, mobile, tel, telephone |
| `email` | email, email address |
| `gender` | gender, sex |
| `unit` | unit, team, team unit |
| `zone` | zone, aisle, zone aisle, area |
| `position` | position, role |

**200**
```json
{
  "importId": "imp_1",
  "fileName": "members-oct.csv",
  "columns": { "First Name": "firstName", "Phone": "phone", "Zone": "zone" },
  "rows": [
    { "row": 2, "data": { "firstName": "Tobi", "lastName": "Adeleke", "phone": "08135550101", "zone": "Aisle 3", "position": "Control 2" },
      "status": "new | update | error", "errors": [], "warnings": ["Zone not found: Aisle 11"], "existingMemberId?": "m_9" }
  ],
  "summary": { "total": 58, "new": 50, "update": 6, "error": 2 }
}
```

Row errors: missing name, missing or invalid phone, phone repeated within the file. Warnings: an unknown zone or position (the row imports without an assignment), or a spot position that is already taken.

### 12.3 `POST /imports/{importId}/commit`

Body `{ "updateExisting": true, "assign": true }`. **200** `{ "added": 50, "updated": 6, "skipped": 2, "assigned": 48 }`. Writes activity and a notification. Previews expire after one hour.

---

## 13. Admin: reports and exports

### `GET /events/{eventId}/exports/{kind}?format=csv|xlsx|pdf&zoneId=`

Returns a file with a `Content-Disposition` filename such as `TGA-2026-full-roster.csv`.

| `kind` | Rows | Columns |
|---|---|---|
| `full` | Every assignment, ordered by zone order then position order | Zone, Position, Duty, Spot, Member, Phone, Team Lead, Status |
| `aisle` | One zone (needs `zoneId`), for leads to print | Same as `full` |
| `unassigned` | Active members with no assignment in the event | Member, Phone, Email, Unit |
| `event` | Every assignment with the unit | Same as `full`, plus Unit |
| `contacts` | All members | Member, Phone, Email, Unit, Status |
| `absences` | Unavailable assignments | Zone, Position, Member, Phone, Reason, Covered |

Rosters (`full`, `aisle`, `event`) leave out unavailable assignments. Put the session (e.g. "Day 1 · Morning session") and its reporting and start times in the PDF header and the file name (`TGA-2026-D1-morning-full-roster.csv`).

The PDF should be a printable A4 table with the event name, date and reporting time in the header.

### `GET /events/{eventId}/reports/summary`

Feeds the health cards on the Reports page: `{ stats: EventStats, byKind: { aisle: {assigned, required}, section: {…}, door: {…}, area: {…} }, byDuty: { … as in §11 }, spots: { total: 312, filled: 290, needsCover: 5 }, lowestCoverage: [5 ZoneStat] }`.

---

## 14. Admin: settings

| Method | Path | Notes |
|---|---|---|
| `GET` | `/settings` | `Settings`. |
| `PATCH` | `/settings` | Partial `Settings`. Validate `contactPhone` as a Nigerian number. |
| `POST` | `/admin/reset-demo` | **Development only.** Reseeds the demo data. Must not exist in production. |

---

## 15. Business rules the server must enforce

The frontend applies these too, but the server is the source of truth.

### 15.1 Phone numbers
Strip everything but digits. `234XXXXXXXXXX` (13 digits) becomes `0XXXXXXXXXX`. A 10-digit number starting with 7, 8 or 9 gets a leading 0. A valid number matches `^0[789][01]\d{8}$` (11 digits, e.g. `0803…`, `0705…`, `0911…`). Store and compare only the normalised form.

### 15.2 One assignment per member per event (session)
Assigning a member who already has an assignment in that session **replaces** it, including an unavailable one. Return the previous one in `replaced.previousOfMember`. A member can, and usually will, have one assignment in each session of a series.

### 15.3 One usher per spot
A position with `spot` set holds one member who is not unavailable. Assigning someone to an occupied spot moves the current occupant off it, which leaves them unassigned, and is returned in `replaced.displacedFromSpot`. If the request has `replace: false`, refuse with **409** `SPOT_TAKEN` instead. Spot positions always have `required = 1`, and only sections with spots (§3.2) may have spot positions.

### 15.4 One team lead per zone
Setting `isTeamLead: true` clears the flag from anyone else in that zone. An unavailable assignment can't be a lead. Assigning someone to a position with `isLeadRole` sets `isTeamLead` by default.

### 15.5 Publishing controls visibility
While `rosterStatus` is `draft`, public endpoints return `unpublished` and no positions, and the public map endpoint returns **404**.

### 15.5a Absences
An unavailable assignment never counts towards coverage, spot occupancy, team lists, rosters or "Ushers serving". Only the member themself (by phone, §4.6), or an admin, can mark it. Undo is refused once someone else holds the seat (§4.7).

### 15.5b Team scope
Team tokens (§5a) only see zones in their scope: a coordinator's own `coordinates`, or a leader's coordinators' `coordinates`. Every team endpoint filters on the server, never only in the frontend.

### 15.6 Unique map references
Within one event, a `mapRef` may belong to only one zone.

### 15.7 Activity log
Record an Activity on every one of these, with the admin's first name as `actor`:
* creating, editing, duplicating, archiving or deleting an event
* publishing or unpublishing a roster
* assigning, moving or removing people (one entry per bulk action, e.g. "assigned 12 ushers to Aisle 4")
* adding, editing or deleting members
* imports
* zone and position changes
* an usher marking themself unavailable, or undoing it (kind `absence`, actor the usher)
* team role changes (kind `team`)
* copying a roster between sessions

### 15.8 Deleting cascades
Deleting an event removes its zones, positions and assignments. Deleting a zone removes its positions and assignments. Deleting a member removes their assignments.

### 15.9 Concurrency
Several admins may assign at once. Use a unique constraint on `(eventId, memberId)` for assignments, and a partial unique constraint on `positionId` for spot positions where `status <> 'unavailable'`, so two admins can't fill the same seat while an absent usher's row is kept. Return **409** on a lost race so the UI can refresh.

---

## 15a. Open questions for the backend developer

1. **Team sign-in.** §5a asks for a one-time code. If SMS is not possible at first, WhatsApp or a PIN set by the admin will do, but the phone number alone must not be enough in production.
2. **Reporting times.** The demo uses 07:30 (morning) and 15:45 (evening). The department sets the real ones on each session; nothing is hard-coded.
3. **Lookup privacy.** Today anyone who types a member's phone number sees that member's post and, if `showTeammatePhones` is on, their teammates' numbers. Consider a one-time code by SMS or WhatsApp before `POST /public/lookup` returns a `found` result (`POST /public/lookup/otp` then `POST /public/lookup` with `{ phone, code }`). The frontend can add the code step when the API supports it.
4. **Admin accounts.** Who creates admin users, and do we need the `editor` role from day one?
5. **Coordinators covering absences.** The team view is read-only for now. If coordinators should fill gaps themselves, allow `POST /events/{eventId}/assignments` with a team token, limited to their zones.
6. **One venue or many.** Events at other venues (e.g. Eagle Square) currently reuse the auditorium plan. If other venues need their own plans, add a `venueId` to Event and serve the plan per venue.

---

## 16. Endpoint index

**Public**
```
GET    /public/current-event
POST   /public/lookup
GET    /public/events/{eventId}/map
POST   /public/assignments/{assignmentId}/confirm
POST   /public/assignments/{assignmentId}/unavailable
POST   /public/assignments/{assignmentId}/available
GET    /public/settings
```

**Team (coordinators and leaders)**
```
POST   /team/auth/request-code
POST   /team/auth/verify
POST   /team/auth/logout
GET    /team/me
GET    /team/events/{eventId}/map
```

**Auth**
```
POST   /auth/login
POST   /auth/refresh
POST   /auth/logout
GET    /auth/me
POST   /auth/forgot-password
POST   /auth/reset-password
```

**Events**
```
GET    /events
POST   /events
GET    /events/{id}
PATCH  /events/{id}
DELETE /events/{id}
POST   /events/{id}/duplicate
POST   /events/{id}/archive
POST   /events/{id}/publish
POST   /events/{id}/unpublish
PUT    /settings/current-event
```

**Zones and positions**
```
GET    /events/{eventId}/zones?include=positions,stats
POST   /events/{eventId}/zones
PUT    /events/{eventId}/zones/order
POST   /events/{eventId}/zones/apply-template
PATCH  /zones/{zoneId}
DELETE /zones/{zoneId}
POST   /zones/{zoneId}/positions
PUT    /zones/{zoneId}/positions/order
PATCH  /positions/{positionId}
DELETE /positions/{positionId}
GET    /venue/template
```

**Members**
```
GET    /members
POST   /members
GET    /members/check-phone
GET    /members/{id}
PATCH  /members/{id}
DELETE /members/{id}
POST   /members/bulk-delete
PUT    /members/{id}/role
GET    /team
GET    /zones/names
```

**Assignments and map**
```
GET    /events/{eventId}/assignments
POST   /events/{eventId}/assignments
POST   /events/{eventId}/assignments/bulk
POST   /events/{eventId}/assignments/bulk-remove
POST   /events/{eventId}/assignments/copy
POST   /events/{eventId}/roster/copy-from
GET    /events/{eventId}/absences
PATCH  /assignments/{id}
DELETE /assignments/{id}
POST   /assignments/{id}/confirm
GET    /events/{eventId}/map
```

**Dashboard, search, activity, notifications**
```
GET    /events/{eventId}/overview
GET    /search
GET    /activity
GET    /notifications
POST   /notifications/read-all
POST   /notifications/{id}/read
```

**Import, reports, settings**
```
GET    /imports/template
POST   /imports/preview
POST   /imports/{importId}/commit
GET    /events/{eventId}/exports/{kind}
GET    /events/{eventId}/reports/summary
GET    /settings
PATCH  /settings
POST   /admin/reset-demo        (development only)
```

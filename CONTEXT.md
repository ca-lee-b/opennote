# OpenNote

OpenNote captures audio recordings, transcribes them into timestamped transcript lines, and keeps the saved transcript available in the local library.

## Language

**Recording**:
A saved audio item and its transcript metadata in the local library.
_Avoid_: Note, session

**Capture session**:
The in-progress workflow that loads the selected model, captures audio, transcribes the saved audio file, and returns transcript lines for a recording.
_Avoid_: Active recording hook, recorder flow

**Imported audio**:
A user-selected existing audio file used to create a recording.
_Avoid_: Upload session, imported recording

**Transcript line**:
A timestamped piece of transcribed text that belongs to one recording.
_Avoid_: Segment, chunk

**Recording write**:
The persistence path that creates or mutates recordings and keeps transcript lines, timestamps, and full text consistent.
_Avoid_: Save helper, database wrapper

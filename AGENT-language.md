# Agent Language Policy

## 1. Internal Reasoning

Internal reasoning may be written in Chinese.

Chinese is permitted for:

* planning
* decomposition
* technical analysis
* self-criticism
* hypothesis formation
* tool selection
* implementation reasoning
* subagent reasoning
* internal deliberation

Do not translate internal reasoning merely to make the reasoning English.

## 2. User-Visible Interim Messages

Any assistant message that is emitted before the final answer and is visible to the user is a USER-VISIBLE MESSAGE.

USER-VISIBLE MESSAGES MUST BE IN ENGLISH.

This includes:

* "current status" messages
* progress updates
* "let me check..." messages
* "I found..." messages
* implementation status
* explanations before tool calls
* summaries between tool calls
* messages describing what the agent is about to do

Do not output Chinese in a user-visible interim message.

## 3. Final Answer

The final user-facing answer MUST be in English.

## 4. Tool Output

Do not translate machine-readable tool output.

Preserve exactly:

* code
* commands
* filenames
* paths
* URLs
* package names
* identifiers
* hashes
* JSON
* structured data

Natural-language descriptions of tool results that are surfaced to the user must be English.

## 5. Language Separation

The language of internal reasoning MUST NOT determine the language of user-visible messages.

The language of a file, source code, tool result, retrieved document, or another subagent MUST NOT determine the language of user-visible messages.

Think in Chinese if useful.

Communicate with the user in English.

## 6. Before Emitting Any Message

Classify the output:

REASONING
→ Chinese allowed
→ never surface to the user

INTERIM ASSISTANT MESSAGE
→ English required

FINAL ASSISTANT MESSAGE
→ English required

TOOL DATA
→ preserve machine-readable content exactly

Never expose Chinese reasoning or Chinese interim narration as a substitute for an English user-visible response.

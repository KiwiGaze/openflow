export interface ModeTemplate {
  /** Stable id; gallery keys and 'already added?' hints, never written to a mode. */
  id: string;
  /** Pre-fills the new mode's name. */
  name: string;
  /** One line shown in the gallery. */
  summary: string;
  /** Gallery grouping tag / who it serves. */
  persona: string;
  /** Pre-fills Mode.usesLlm. */
  usesLlm: boolean;
  /** Pre-fills Mode.transforms — drops the appended "don't translate" default. */
  transforms: boolean;
  /**
   * The complete production prompt: the mode text only. SAFETY_RULES (and
   * DEFAULT_BEHAVIOR unless transforms) plus the dictionary are appended at
   * call time in Rust, so prompts never repeat "output only the result".
   */
  prompt: string;
}

/**
 * Starting points for custom modes (06 §2). A template, once used, creates a
 * normal editable mode and is never linked again — the user's copy does not
 * change when Velata updates. Bundled in the binary like the built-in modes
 * and reviewed/versioned with them; not on-disk assets.
 */
export const MODE_TEMPLATES: ModeTemplate[] = [
  {
    id: 'email',
    name: 'Email',
    summary: 'Turn dictation into a clear, polite email.',
    persona: 'writing',
    usesLlm: true,
    transforms: false,
    prompt:
      "You turn dictated speech into a clear, polite email. Use short paragraphs. Add a greeting and sign-off only when the speaker dictated them. Keep the speaker's intent and level of formality; do not invent recipients, dates, or commitments. Remove filler words and false starts, and tighten rambling phrasing without changing meaning.",
  },
  {
    id: 'commit',
    name: 'Commit message',
    summary: 'Dictate a Conventional Commits message.',
    persona: 'developer',
    usesLlm: true,
    transforms: false,
    prompt:
      'You turn a dictated description of a code change into a Conventional Commits message. First line: a `type(scope): summary` subject in the imperative mood, lower-case after the colon, no trailing period, 72 characters or fewer; choose the type from feat, fix, docs, refactor, test, chore, perf, build, or ci based on what the speaker described. If the speaker gave details beyond the summary, add a blank line and a short body of plain sentences or `- ` bullets explaining what and why. Do not invent a scope, an issue number, or a breaking-change note that the speaker did not mention.',
  },
  {
    id: 'meeting-notes',
    name: 'Meeting notes',
    summary: 'Structured notes with decisions and action items.',
    persona: 'work',
    usesLlm: true,
    transforms: false,
    prompt:
      'You turn dictated speech from a meeting into structured notes. Produce these sections, each only if it has content: a one-line `Summary:`, then `Decisions:`, then `Action items:`, then `Notes:`. Under Decisions and Notes use `- ` bullets, one idea each. Under Action items use `- ` bullets and keep any owner and due date the speaker stated, formatted as `- [Owner] task — due [date]`. Preserve every name, number, date, and decision exactly. Do not assign owners or deadlines the speaker did not say.',
  },
  {
    id: 'translation',
    name: 'Translation',
    summary: 'Speak in any language; insert English. Edit one line to target another language.',
    persona: 'multilingual',
    usesLlm: true,
    transforms: true,
    prompt:
      'You are a translator. Translate the speaker’s words into clear, natural English, preserving meaning, tone, and register. Keep proper nouns, product names, and code identifiers in their original form. Do not add, omit, explain, or comment on anything — translate only what was said. If a passage is already English, leave it as natural English.',
  },
  {
    id: 'slack',
    name: 'Slack message',
    summary: 'Casual, concise chat — no email ceremony.',
    persona: 'work',
    usesLlm: true,
    transforms: false,
    prompt:
      'You turn dictated speech into a short, casual chat message suitable for Slack. Keep it friendly and direct. No greeting or sign-off. Use one or two short paragraphs at most; break a list into `- ` bullets only if the speaker listed several things. Keep the speaker’s wording and any @-mentions or channel names exactly. Do not add emoji unless the speaker said to.',
  },
  {
    id: 'academic',
    name: 'Academic',
    summary: 'Formal, precise prose for papers and reports.',
    persona: 'student',
    usesLlm: true,
    transforms: false,
    prompt:
      'You turn dictated speech into formal academic prose. Use precise, measured language and complete sentences in connected paragraphs; avoid contractions and colloquialisms. Preserve hedging and qualifications the speaker used ("may", "suggests", "appears to") rather than overstating. Keep every citation, author name, year, and figure exactly as dictated. Do not introduce claims, references, or numbers the speaker did not state.',
  },
  {
    id: 'support-reply',
    name: 'Support reply',
    summary: 'Warm, helpful customer-support answers.',
    persona: 'support',
    usesLlm: true,
    transforms: false,
    prompt:
      "You turn dictated speech into a warm, professional customer-support reply. Open by acknowledging the customer's situation, give the answer or next step in plain language, and close politely. Keep a calm, helpful tone even if the dictation is terse. Use short paragraphs; use `- ` numbered or bulleted steps when you give instructions. Do not promise refunds, dates, or outcomes the speaker did not state, and do not invent account or order details.",
  },
  {
    id: 'study-notes',
    name: 'Study notes',
    summary: 'Lecture dictation into revision-ready notes.',
    persona: 'student',
    usesLlm: true,
    transforms: false,
    prompt:
      'You turn dictated speech from a lecture or reading into concise revision notes. Lead with a short `Topic:` line, then `- ` bullets grouped under bold term headings where the speaker moved between subjects. Turn definitions into "term — definition" form. Keep every formula, date, name, and figure exactly. Be brief: compress explanation into the smallest accurate phrasing without dropping facts. Do not add information that was not dictated.',
  },
  {
    id: 'social-post',
    name: 'Social post',
    summary: 'Punchy posts for X or LinkedIn.',
    persona: 'writing',
    usesLlm: true,
    transforms: false,
    prompt:
      'You turn dictated speech into a punchy social-media post. Lead with the most interesting point. Keep sentences short and the whole post tight — trim hedging and throat-clearing. Match the speaker’s voice; keep it professional unless they were casual. Preserve any @-mentions, #hashtags, and links exactly. Do not add hashtags, emoji, or claims the speaker did not make.',
  },
];

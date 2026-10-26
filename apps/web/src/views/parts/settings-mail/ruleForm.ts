import type {
  DecisionPointKey,
  MailRuleInput,
  MailRuleView,
  RuleCondition,
  RuleField,
  RuleOp,
} from '../../../api/mailSettings'

export const DECISION_POINTS: { key: DecisionPointKey; label: string }[] = [
  { key: 'email.category', label: 'Category' },
  { key: 'email.time_sensitive', label: 'Time-sensitive' },
  { key: 'email.member_link', label: 'Who it is about' },
  { key: 'email.is_bill', label: 'Is a bill' },
  { key: 'event.calendar_route', label: 'Calendar for an event' },
]

export const FIELDS: { key: RuleField; label: string }[] = [
  { key: 'from_addr', label: 'From address' },
  { key: 'from_domain', label: 'From domain' },
  { key: 'to', label: 'To' },
  { key: 'subject', label: 'Subject' },
  { key: 'body', label: 'Body' },
  { key: 'gmail_label', label: 'Gmail label' },
  { key: 'header', label: 'Header' },
  { key: 'has_attachment_type', label: 'Attachment type' },
  { key: 'org_kind', label: 'Organization kind' },
  { key: 'org', label: 'Organization' },
  { key: 'source', label: 'Mail source' },
]

export const OPS: { key: RuleOp; label: string }[] = [
  { key: 'is', label: 'is' },
  { key: 'in', label: 'is one of' },
  { key: 'contains_any', label: 'contains any of' },
  { key: 'matches', label: 'matches (regex)' },
  { key: 'exists', label: 'exists' },
]

export interface ConditionRow {
  field: RuleField
  op: RuleOp
  header: string
  /** Comma separated in the UI; one value per line is also accepted. */
  values: string
}

export interface RuleForm {
  name: string
  decision_point: DecisionPointKey
  priority: number
  confidence: number
  enabled: boolean
  conditions: ConditionRow[]
  /** Category key, member id, calendar id, or 'true' / 'false'. */
  value: string
}

export const emptyCondition = (): ConditionRow => ({
  field: 'from_domain',
  op: 'in',
  header: '',
  values: '',
})

export const emptyRuleForm = (): RuleForm => ({
  name: '',
  decision_point: 'email.category',
  priority: 100,
  confidence: 0.9,
  enabled: true,
  conditions: [emptyCondition()],
  value: '',
})

export function splitValues(s: string): string[] {
  return s
    .split(/[,\n]/)
    .map((x) => x.trim())
    .filter(Boolean)
}

export const isBooleanPoint = (dp: DecisionPointKey) =>
  dp === 'email.time_sensitive' || dp === 'email.is_bill'

export function toCondition(r: ConditionRow): RuleCondition {
  return {
    field: r.field,
    op: r.op,
    ...(r.field === 'header' ? { header: r.header.trim() } : {}),
    values: r.op === 'exists' ? [] : splitValues(r.values),
  }
}

/** The wire value for the decision point: booleans for the yes/no points, strings otherwise. */
export function toValue(dp: DecisionPointKey, v: string): unknown {
  return isBooleanPoint(dp) ? v !== 'false' : v
}

export function toInput(f: RuleForm): MailRuleInput {
  return {
    decision_point: f.decision_point,
    name: f.name.trim(),
    priority: f.priority,
    conditions: f.conditions.map(toCondition),
    value: toValue(f.decision_point, f.value) as MailRuleInput['value'],
    confidence: f.confidence,
    enabled: f.enabled,
  }
}

export function fromView(r: MailRuleView): RuleForm {
  return {
    name: r.name,
    decision_point: r.decision_point,
    priority: r.priority,
    confidence: r.confidence,
    enabled: r.enabled,
    conditions: r.conditions.map((c) => ({
      field: c.field,
      op: c.op,
      header: c.header ?? '',
      values: (c.values ?? []).join(', '),
    })),
    value: typeof r.value === 'string' ? r.value : String(r.value),
  }
}

/** Plain-words summary of a rule's conditions for the list. */
export function describeConditions(cs: RuleCondition[]): string {
  const f = (k: string) => FIELDS.find((x) => x.key === k)?.label ?? k
  const o = (k: string) => OPS.find((x) => x.key === k)?.label ?? k
  return cs
    .map(
      (c) =>
        `${f(c.field)}${c.header ? ` ${c.header}` : ''} ${o(c.op)}${(c.values ?? []).length ? ` ${(c.values ?? []).join(', ')}` : ''}`,
    )
    .join(' and ')
}

export type Visibility = 'full' | 'busy'

export interface EventLike {
  id?: string
  title?: string
  location?: string | null
  start_at: string
  end_at: string
  all_day: boolean
  /** ISO date (YYYY-MM-DD), set for all-day events. */
  start_date?: string | null
  /** ISO date, exclusive. */
  end_date?: string | null
  members: string[]
  color: string
  /** More than one: the event is merged from several calendars; the stripe shows each. */
  colors?: string[]
  visibility: Visibility
  source_url?: string | null
}

export interface MemberLike {
  id: string
  display_name: string
  color: string
  avatar_url?: string | null
}

export type UnitSystem = 'metric' | 'imperial'

export type RangeMode = 'day' | 'week' | 'month'

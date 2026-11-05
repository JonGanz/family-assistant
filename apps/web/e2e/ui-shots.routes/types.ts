import type { Page } from '@playwright/test'

/** One screenshot: visit `route`, optionally act (open a sheet, pick a tab), then capture as `name`. */
export interface ShotRoute {
  route: string
  /** File name stem; defaults to the route slug. */
  name?: string
  /** Runs before the visit (install `page.route` mocks here). */
  setup?: (page: Page) => Promise<void>
  /** Runs after the page settles and before the capture. */
  before?: (page: Page) => Promise<void>
  /** Only at these widths (default: all). */
  widths?: number[]
}

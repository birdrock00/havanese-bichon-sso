import { useLayoutEffect, useRef, useState } from 'react'
import { cn } from '@/lib/utils'

type AutoFitTextProps = {
  text: string
  className?: string
  /** Native tooltip (usually the full text) for extreme overflow cases. */
  title?: string
  /** Smallest font size (px) to shrink to before letting it clip. */
  minSize?: number
  /** Font size (px) used when the text fits without shrinking. */
  maxSize?: number
}

/**
 * Renders a single line of text that shrinks its font size so the full
 * content stays visible inside its container (e.g. long brand names in the
 * sidebar). Re-measures whenever the container resizes or the text changes.
 * Below `minSize` the text clips, so pair with a `title` tooltip for extremes.
 */
export function AutoFitText({
  text,
  className,
  title,
  minSize = 10,
  maxSize = 16,
}: AutoFitTextProps) {
  const containerRef = useRef<HTMLSpanElement>(null)
  const textRef = useRef<HTMLSpanElement>(null)
  const [fontSize, setFontSize] = useState(maxSize)

  useLayoutEffect(() => {
    const container = containerRef.current
    const el = textRef.current
    if (!container || !el) return

    const fit = () => {
      let size = maxSize
      // Leave a small slack so rounding doesn't clip the last few pixels.
      const availWidth = container.clientWidth - 2
      el.style.fontSize = `${size}px`
      while (el.scrollWidth > availWidth && size > minSize) {
        size -= 1
        el.style.fontSize = `${size}px`
      }
      setFontSize(size)
    }

    fit()

    const observer = new ResizeObserver(fit)
    observer.observe(container)
    return () => observer.disconnect()
  }, [text, minSize, maxSize])

  return (
    <span
      ref={containerRef}
      title={title}
      className={cn('min-w-0 overflow-hidden', className)}
    >
      <span
        ref={textRef}
        className='block whitespace-nowrap font-semibold'
        style={{ fontSize }}
      >
        {text}
      </span>
    </span>
  )
}

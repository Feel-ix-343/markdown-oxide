# Date Formatting in Markdown Oxide

Markdown Oxide uses the [Chrono](https://docs.rs/chrono/latest/chrono/) Rust library for date formatting. This allows you to customize how dates appear in your daily notes and other date-related features.

## Configuration

You can configure the date format pattern in your `.moxide` configuration file:

```yaml
dailynote: "%Y-%m-%d" # Default format: 2024-07-15
```

## Format Specifiers

Markdown Oxide supports the standard Chrono date format specifiers, which are similar to strftime patterns used in many programming languages.

### Common Format Specifiers

#### Year

| Specifier | Description  | Example |
| --------- | ------------ | ------- |
| `%Y`      | 4-digit year | 2024    |
| `%y`      | 2-digit year | 24      |

#### Month

| Specifier | Description                | Example |
| --------- | -------------------------- | ------- |
| `%m`      | Zero-padded month          | 07      |
| `%-m`     | Month without leading zero | 7       |
| `%B`      | Full month name            | July    |
| `%b`      | Abbreviated month name     | Jul     |

#### Day

| Specifier | Description                       | Example |
| --------- | --------------------------------- | ------- |
| `%d`      | Zero-padded day of month          | 05      |
| `%-d`     | Day of month without leading zero | 5       |
| `%A`      | Full weekday name                 | Monday  |
| `%a`      | Abbreviated weekday name          | Mon     |

### Example Formats

| Description                   | Format String | Example Output  |
| ----------------------------- | ------------- | --------------- |
| ISO format (default)          | `%Y-%m-%d`    | 2024-07-15      |
| US style with full month      | `%B %d, %Y`   | July 15, 2024   |
| Short format with abbr. month | `%d %b %Y`    | 15 Jul 2024     |
| Day-first format              | `%d/%m/%Y`    | 15/07/2024      |
| With weekday                  | `%A, %B %d`   | Monday, July 15 |

## Obsidian Compatibility

If you're migrating from Obsidian, Markdown Oxide automatically converts Moment.js date formats (used by Obsidian) to Chrono format.

For full documentation on all available format specifiers, see the [Chrono documentation](https://docs.rs/chrono/latest/chrono/format/strftime/index.html).

---

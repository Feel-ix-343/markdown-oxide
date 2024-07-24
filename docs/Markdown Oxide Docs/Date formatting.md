

The date formatting in the config follows the rust library Chrono's formatting. 

The full specification can be found here: https://docs.rs/chrono/latest/chrono/format/strftime/index.html

Some examples are:

- Year
    * `%Y`: Is the four digit year
    * `%y`: Is the two digit year: 1979 -> 79
- Month
    * `%m`: Is the two digit month
    * `%b`: Is the abbreviated month name; 3 letters
    * `%B`: Full moth name or 3 letter abbreviation
- Day
    * `%d`: Is the two digit day of month
    * `%e`: Is the one or two digit day of month
    * `%a`: 3 letter abbreviated weekday name
    * `%A`: Abbreviated or full weekday name



+ =>
    * (the default) `YYYY-MM-DD` -> `%Y-%m-%d`
    * "1 Jan 2024" => `%d %b %Y`

---

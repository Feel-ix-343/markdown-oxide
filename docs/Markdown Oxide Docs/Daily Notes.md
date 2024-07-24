




# Relative Names

## Completion Names

There are a set of predefined relative names for referencing your daily notes. 

Here it is: 

![[Daily Notes#^predefinedNames]]

These can be used in completions by `[[{relativename}` or `[{display?}]({relativename}`. 

- For example
    * write `[[today` + select completion -> `[[2024-07-14]]`
    * write `[[tomorrow` + select completion -> `[[2024-07-15]]`
    * write `[](tomorrow` + select completion -> `[tomorrow](2024-07-15)`
    * write `[Tomorrow](tomorrow` + select completion -> `[Tomorrow](2024-07-15)`
        + notice that the display text was not changed

## Opening Daily Notes

These follow https://docs.rs/fuzzydate/latest/fuzzydate/ where the relative name must follow the specification for `<datetime>` in...

- <details><summary>fuzzydate's specification</summary>

    ```
    <datetime> ::= <time>
                 | <date> <time>
                 | <date> , <time>
                 | <duration> after <datetime>
                 | <duration> from <datetime>
                 | <duration> before <datetime>
                 | <duration> ago
                 | now

    <article> ::= a
               | an
               | the

    <date> ::= today
             | tomorrow
             | yesterday
             | <num> / <num> / <num>
             | <num> - <num> - <num>
             | <num> . <num> . <num>
             | <month> <num> <num>
             | <relative_specifier> <unit>
             | <relative_specifier> <weekday>
             | <weekday>

    <relative_specifier> ::= this
                           | next
                           | last

    <weekday> ::= monday
                | tuesday
                | wednesday
                | thursday
                | friday
                | saturday
                | sunday
                | mon
                | tue
                | wed
                | thu
                | fri
                | sat
                | sun

    <month> ::= january
              | february
              | march
              | april
              | may
              | june
              | july
              | august
              | september
              | october
              | november
              | december
              | jan
              | feb
              | mar
              | apr
              | jun
              | jul
              | aug
              | sep
              | oct
              | nov
              | dec

    <duration> ::= <num> <unit>
                 | <article> <unit>
                 | <duration> and <duration>

    <time> ::= <num>:<num>
             | <num>:<num> am
             | <num>:<num> pm
             |

    <unit> ::= day
             | days
             | week
             | weeks
             | hour
             | hours
             | minute
             | minutes
             | min
             | mins
             | month
             | months
             | year
             | years

    <num> ::= <num_triple> <num_triple_unit> and <num>
            | <num_triple> <num_triple_unit> <num>
            | <num_triple> <num_triple_unit>
            | <num_triple_unit> and <num>
            | <num_triple_unit> <num>
            | <num_triple_unit>
            | <num_triple>
            | NUM   ; number literal greater than or equal to 1000

    <num_triple> ::= <ones> hundred and <num_double>
                   | <ones> hundred <num_double>
                   | <ones> hundred
                   | hundred and <num_double>
                   | hundred <num_double>
                   | hundred
                   | <num_double>
                   | NUM    ; number literal less than 1000 and greater than 99

    <num_triple_unit> ::= thousand
                        | million
                        | billion

    <num_double> ::= <ones>
                   | <tens> - <ones>
                   | <tens> <ones>
                   | <tens>
                   | <teens>
                   | NUM    ; number literal less than 100 and greater than 19

    <tens> ::= twenty
             | thirty
             | forty
             | fifty
             | sixty
             | seventy
             | eighty
             | ninety

    <teens> ::= ten
              | eleven
              | twelve
              | thirteen
              | fourteen
              | fifteen
              | sixteen
              | seventeen
              | eighteen
              | nineteen
              | NUM     ; number literal less than 20 and greater than 9

    <ones> ::= one
             | two
             | three
             | four
             | five
             | six
             | seven
             | eight
             | nine
             | NUM      ; number literal less than 10
    ```

</details>


Instead of memorizing this, however, I'd recommend just trying relative names out. 

### Neovim

Here are some examples for neovim daily note commands (as specified in the setup [here](README#^nvimconfigsetup)

- examples ^nvimrelativenamescmds
    * `:Daily two days ago`
    * `:Daily 2 days ago`
    * `:Daily next monday`
    * `:Daily last friday`
    * `:Daily today`
    * `:Daily tomorrow`

### Other

The neovim configuration uses the an [LSP Workspace Command]() called Daily accepting `arguments` as a string, which is the relative name. When not specified, the daily command will take you to today's note. 

In editors that support workspace commands with arguments, you may be able to configure them to work like neovim. (With [[Editor Specific Plugins]], one day this configuration may be done for you.)

For editors that support commands but not arguments, there is a set of predefined commands for navigating daily notes. Here is the list:



- Predefined Relative Names: ^predefinedNames
    * `today`
    * `tomorrow`
    * `yesterday`
    * `next {monday,tuesday,..., sunday}`
    * `last {monday,tuesday,...}`

## 

In the future we hope to also support jumping relative to an active opened note. For example, there would be a `prev dailynote` and a `next dailynote`. The issue for this can be found [here](https://github.com/Feel-ix-343/markdown-oxide/issues/101)




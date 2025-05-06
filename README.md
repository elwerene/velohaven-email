This project is meant to run once a day via a cron job. It will download the current list of email addresses and templates and send out emails to each address using the template. The email will be sent using the SMTP server specified in the configuration file.

# Configuration

The configuration file is a toml file that contains the following fields:
- `server`: The Nextcloud server hostname
- `username`: The username to use for authentication (cryptic part of share url)
- `email`: The email configuration
- `min_date`: Emails which where added before this date will treated as added on this date. The date format is `YYYY-MM-DD`


## Email configuration

The email configuration is contains the following fields:
- `host`: The SMTP server hostname
- `username`: The username to use for authentication
- `password`: The password to use for authentication
- `from`: The email address to use as the sender
- `to_overwrite` (Optional): The email address to use as the recipient. If this is not set, the email will be sent to the address specified in the address list.

## Example configuration

```toml
server = "nextcloud.example.com"
username = "Token from share link"

[email]
from = "from@example.com"
to_overwrite = "dev@example.com"
host = "mail.example.com"
username = "dev"
password = "dev123"
```

# Nextcloud folder format

The Nextcloud folder should contain the following files/folders:
- `emails.txt`: A text file containing the list of email addresses and dates. Each line should contain an email address and a date in the format `YYYY-MM-DD`
- `templates`: A folder containing the email templates

## Template

Each template should be a html file with the name of the template.

Format of the name is `{date offset} - {subject}.html`. The date offset can be any offset as understood by [parse_duration](https://docs.rs/parse_duration/latest/parse_duration/)

The template contains the body which will be sent to the email address.

### Example template names:

 * `1 day - Welcome.html` - is send on the day the user is added. The subject is `Welcome`.
 * `1 week - Reminder.html` - is send 1 week after the user is added. The subject is `Reminder`.
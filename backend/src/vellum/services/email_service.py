import os
import smtplib
from email.mime.multipart import MIMEMultipart
from email.mime.base import MIMEBase
from email.mime.text import MIMEText
from email import encoders

class EmailService:
    @staticmethod
    def send_to_kindle(
        smtp_host: str,
        smtp_port: int,
        smtp_user: str,
        smtp_pass: str,
        kindle_email: str,
        book_filepath: str,
        use_tls: bool = True
    ) -> bool:
        if not os.path.exists(book_filepath):
            raise Exception("Book file not found.")

        filename = os.path.basename(book_filepath)
        msg = MIMEMultipart()
        msg['From'] = smtp_user
        msg['To'] = kindle_email
        msg['Subject'] = f"Book Transfer: {filename}"

        msg.attach(MIMEText("Voici votre livre envoyé depuis Vellum Ebook Library.", "plain"))

        with open(book_filepath, "rb") as attachment:
            part = MIMEBase("application", "octet-stream")
            part.set_payload(attachment.read())
            encoders.encode_base64(part)
            part.add_header("Content-Disposition", f"attachment; filename={filename}")
            msg.attach(part)

        if use_tls:
            server = smtplib.SMTP(smtp_host, smtp_port, timeout=15)
            server.starttls()
        else:
            server = smtplib.SMTP_SSL(smtp_host, smtp_port, timeout=15)

        server.login(smtp_user, smtp_pass)
        server.send_message(msg)
        server.quit()
        return True

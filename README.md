# cloud_sync
Sync mechanism between OneDrive Personal and AWS S3

## How it operates
OneDrive has APIs for getting deltas since last delta request. This list of deltas is based upon file last modification date.
The first time cloud_sync is run it thus gets all files currently in the OneDrive account, after that cloud_sync saves a specific
delta link in a file called delta_link.json. If for any reason a full re-sync is needed, just remove that delta_link.json file.

Each item in the delta list has amongst others a name (which is the full path to the file) and a last modification datetime.
Unfortunately the last modification date in AWS S3 is rather when the file was put there so it will differ from OneDrive.
To cope with that, cloud_sync adds a metadata tag on the S3 file (or object as they call it) called mtime with the timestamp version of the last modification date from OneDrive as value.
So whenever a sync (full or just single deltas) is fetched from OneDrive, each OneDrive item returned in the delta list is compared with the mtime value for the respective object with the same full path.
Cloud_sync is using the head_object() function to get only metadata from AWS.

If there is a difference, either that the file does not exist at all in the given path, or if the mtime is different, the file is either put in one go to the bucket or
uploaded as a multipart file (depending on the size of the file).

### Important note, and also something that may be improved in later versions
If a file is moved between directories in OneDrive it will be seen as a new file in the delta list and uploaded in the S3 bucket.
Also, if a folder name is changed in OneDrive, that won't be noted as a delta change, but any new file (or modified file) under the new
directory name will again be uploaded with the new path to the S3 bucket.

This can especially be a concern if a directory with a huge amount of files and/or big files is name changed, and a full
re-sync is made by removing the delta_link.json. Because then all those files will be uploaded again but under the new path.

## How to save som money
An AWS S3 bucket can store objects in different storage classes, so if the bucket is used only as for emergency backup, life cycle rules
can be defined so that objects are moved to the Glacier Deep Archive after som days.
Also, a life cycle rule can automatically remove old versions (if versioning is defined for the bucket).
A good practise is to at least make sure that incomplete multipart uploads are deleted after some days.

## Set-up
### Microsoft 
Follow https://learn.microsoft.com/en-us/graph/auth-v2-user?tabs=http to set up
an App in Azure (choose account type: Personal Microsoft accounts only), choose web as platform and as Redirect Url enter https://<host.domain>:8000/code, 
where <host.domain> is your server. Since we are using https we need a TLS certificate and that needs a 
proper domain.

Note down the App id and also create a secret and note down the secret value.  

Under API Permissions chose the following permissions:
 * offline_access
 * Files.Read 
 * Files.Read.All 
 * Files.ReadWrite 
 * Files.ReadWrite.All

### AWS
For the backup, set up an account in AWS and create an S3 standard bucket.
Create an IAM user with permissions for S3 specific, probably enough with read, write and list, but if unsure give S3 Full.
Create an access key for that user and note down the Access Key ID and the Access Key.

For the mail service and the HTTP TLS, create a domain using the AWS service Route53.
Also create an IAM user with permissions for Certbot (see Web Server section below).

### Mail
The application is using the Sendgrid API, so sign up for a free account at Twilio Sendgrid.
Sendgrid has instructions for verifying that the domain to be used belongs to you, follow them and 
make necessary changes in AWS Route53.

Note down the API KEY

### Web Server
The web server in the application needs to run HTTP TLS (HTTPS). Easiest way to get a certificate for the 
host running the application is to use Let's Encrypt and install their Certbot. Cloud_sync assumes that we use
a wildcard cert, so follow the instructions for setting up Certbot for wildcard. 

Certbot needs AWS credentials in the form of an Access Key ID and an Access Key, it can be provided in several ways.
One way is to create an authorization file under the home directory for the user running Certbot (it is explained in the Certbot set-up guide).
If using root (sudo) there is less to do, but also cloud_sync needs to be run under sudo to get access to the certificates. 
Certbot are using the AWS credentials to temporarily create a TXT record on the domain to make sure of correct ownership.

Once the cert is created, DNS A records can be created using AWS Route53. The A record can point to the private IP address range
so the server running cloud_sync doesn't need to be exposed to internet. And since using a wildcard cert, Let's Encrypt does not
have to challenge using an internet open host:port, but rather using DNS-challenge. When ordering a cert from Certbot make sure
to give it as a wildcard, e.g. "*.domain".

After Certbot has created private key and cert for the domain, it shows the path to the created files. 
Enter the path to privkey.pem (tls_private_key) and fullchain.pem (tls_chain_cert) in the cloud_sync config file.

## Running it
Cloud_sync needs one piece of information to start, the path to the config.toml file.
That path is given through the environment variable CONFIG_PATH and the path has to end with a slash (/).

So for example in a linux environment it can be easily run in a bash start script where first the 
CONFIG_PATH is exported and the executable (cloud_sync) is run.

To make it run in the background the start script can be run like e.g. ./start.sh >> /dev/null 2>&1 &

Better solutions can probably easy be found.

### Onedrive authorization
Before cloud_sync can start sync any files it needs a set of access and refresh tokens from Microsoft on 
behalf of you. So, after starting the server, head to https://<host.domain>:<bind_port>/grant where of course <host.domain>
is the server you are running cloud_sync within. You will then end up in an OAuth2.0 Code flow where Microsoft will ask
for you permission to act on your behalf and then send back a code to your defined redirect URL, which in turn will be
traded for access/refresh tokens, which in turn will be saved where you have defined them to be saved.

At some point the refresh token will also expire. Cloud_sync will at that point write an error to the error log and send 
that same error to the mail address defined in the config file.


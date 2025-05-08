# cloud_sync
Sync mechanism between OneDrive Personal and AWS S3

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





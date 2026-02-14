# 📋 Workflow Examples

Real-world examples of how to use clide for common DevOps and system administration tasks.

---

## 🎯 Basic Operations

### Simple Commands

#### Check System Status
```
You: "What's my disk usage?"

clide: 🛫 Checking disk usage...
       /dev/sda1: 68% (42GB/62GB)
       Top consumers:
       - /var/log: 12GB
       - /tmp: 8GB
       - /home: 15GB
       🛬 Done!
```

#### Process Management
```
You: "Show me top 5 memory-consuming processes"

clide: 🛫 Analyzing processes...
       1. nodejs (2.4 GB) - PID 1234
       2. postgres (1.8 GB) - PID 5678
       3. nginx (512 MB) - PID 9012
       4. redis (256 MB) - PID 3456
       5. python (128 MB) - PID 7890
       🛬 Done!
```

#### Network Diagnostics
```
You: "Check if port 80 is open"

clide: 🛫 Checking port 80...
       ✓ Port 80 is OPEN
       ✓ Listening on 0.0.0.0:80
       ✓ Process: nginx (PID 9012)
       🛬 Done!
```

---

## 🔒 VPS Hardening Workflows

### Full Security Audit

```
You: "Harden my VPS to Lynis score 70"

clide: 🛫 Starting security hardening...
       📊 Current Lynis score: 42/100
       
       Step 1/8: Updating system packages
       ✓ 47 packages updated
       
       Step 2/8: Configuring SSH
       ✓ Disabled root login
       ✓ Changed port to 2222
       ✓ Enabled key-only authentication
       
       Step 3/8: Setting up firewall (UFW)
       ✓ Default deny incoming
       ✓ Allow SSH (port 2222)
       ✓ Allow HTTP/HTTPS (80, 443)
       
       Step 4/8: Installing fail2ban
       ✓ Installed and configured
       ✓ Monitoring SSH, HTTP
       
       Step 5/8: Configuring automatic updates
       ✓ unattended-upgrades enabled
       ✓ Security updates daily
       
       Step 6/8: Setting up AppArmor
       ✓ AppArmor enabled
       ✓ Profiles loaded
       
       Step 7/8: Disabling unused services
       ✓ Stopped: cups, avahi-daemon
       
       Step 8/8: Running Lynis audit
       ✓ Final score: 71/100
       
       🛬 Hardening complete!
       📄 Full report: /var/log/lynis-2025-02-15.log
```

### Individual Security Tasks

#### SSH Configuration
```
You: "Make SSH more secure"

clide: 🛫 Securing SSH...
       ✓ Backup created: /etc/ssh/sshd_config.backup
       ✓ Disabled root login
       ✓ Disabled password authentication
       ✓ Changed port to 2222
       ✓ Enabled key-only auth
       ✓ Restarted SSH service
       
       ⚠️  Important: Test SSH connection now!
       New command: ssh -p 2222 user@server
       🛬 Done!
```

#### Firewall Setup
```
You: "Setup firewall for web server"

clide: 🛫 Configuring UFW firewall...
       ✓ UFW installed
       ✓ Default: deny incoming, allow outgoing
       ✓ Allow SSH (port 22)
       ✓ Allow HTTP (port 80)
       ✓ Allow HTTPS (port 443)
       ✓ Firewall enabled
       
       Status:
       22/tcp   ALLOW   Anywhere
       80/tcp   ALLOW   Anywhere
       443/tcp  ALLOW   Anywhere
       🛬 Done!
```

---

## 🐳 Container Management

### Docker Setup

```
You: "Setup PostgreSQL container for my app"

clide: 🛫 Setting up PostgreSQL container...
       
       Step 1: Installing Docker
       ✓ Docker installed
       ✓ Docker service started
       
       Step 2: Creating network
       ✓ Created: app-network
       
       Step 3: Setting up volumes
       ✓ Created: postgres-data
       ✓ Created: postgres-backups
       
       Step 4: Pulling image
       ✓ postgres:16-alpine pulled (89MB)
       
       Step 5: Starting container
       ✓ Container running: postgres-prod
       ✓ Port: 5432
       ✓ Network: app-network
       
       Step 6: Creating database
       ✓ Database: myapp_db created
       ✓ User: myapp_user created
       
       Step 7: Configuring backups
       ✓ Daily backups at 2:00 AM
       ✓ Retention: 7 days
       
       📝 Connection details saved to vault
       
       Connection string:
       postgresql://myapp_user:***@localhost:5432/myapp_db
       
       🛬 Done!
```

### Multi-Container Application

```
You: "Deploy full stack: nginx, node, postgres, redis"

clide: 🛫 Deploying full stack...
       
       ✓ Creating docker-compose.yml
       ✓ nginx:alpine (reverse proxy)
       ✓ node:20-alpine (app server)
       ✓ postgres:16-alpine (database)
       ✓ redis:7-alpine (cache)
       
       ✓ Starting services...
       ✓ All containers healthy
       
       Services:
       - nginx: http://localhost:80
       - node: internal:3000
       - postgres: internal:5432
       - redis: internal:6379
       
       🛬 Stack deployed!
```

---

## 🚀 Application Deployment

### Node.js Application

```
You: "Deploy my Node app from GitHub"

clide: 🛫 Deploying Node.js application...
       
       Step 1: Cloning repository
       ✓ git clone https://github.com/you/app.git
       ✓ Branch: main
       
       Step 2: Installing dependencies
       ✓ npm install (234 packages)
       
       Step 3: Running build
       ✓ npm run build
       ✓ Build output: /dist
       
       Step 4: Setting up environment
       ✓ .env configured
       ✓ Database URL set
       ✓ API keys loaded from vault
       
       Step 5: Starting with PM2
       ✓ pm2 start app.js
       ✓ Process: app (id: 0)
       ✓ Auto-restart enabled
       
       Step 6: Configuring Nginx
       ✓ Reverse proxy configured
       ✓ Domain: app.example.com
       
       Step 7: SSL Certificate
       ✓ Let's Encrypt certificate obtained
       ✓ Auto-renewal configured
       ✓ SSL rating: A+
       
       🛬 App deployed!
       🌐 Live at: https://app.example.com
```

### Static Website

```
You: "Deploy static site with SSL"

clide: 🛫 Deploying static site...
       ✓ Nginx installed
       ✓ Files copied to /var/www/site
       ✓ Virtual host configured
       ✓ SSL certificate obtained (Let's Encrypt)
       ✓ HTTPS redirect enabled
       ✓ Gzip compression enabled
       ✓ Cache headers set
       
       🛬 Site live!
       🌐 https://yoursite.com
```

---

## 🗄️ Database Operations

### Database Backup

```
You: "Backup all databases to S3"

clide: 🛫 Backing up databases...
       
       Found databases:
       - production_db (2.1 GB)
       - staging_db (850 MB)
       - analytics_db (3.4 GB)
       
       ✓ Dumping production_db...
       ✓ Compressing (gzip)
       ✓ Encrypting (GPG)
       ✓ Uploading to S3
       
       ✓ Dumping staging_db...
       ✓ Compressing (gzip)
       ✓ Encrypting (GPG)
       ✓ Uploading to S3
       
       ✓ Dumping analytics_db...
       ✓ Compressing (gzip)
       ✓ Encrypting (GPG)
       ✓ Uploading to S3
       
       Total: 6.4 GB → 1.2 GB (compressed)
       S3 location: s3://backups/db-2025-02-15/
       
       🛬 Backup complete!
```

### Database Migration

```
You: "Migrate database from MySQL to PostgreSQL"

clide: 🛫 Starting database migration...
       
       [DRY-RUN] This will:
       1. Export MySQL data
       2. Convert schema
       3. Import to PostgreSQL
       4. Verify data integrity
       
       Estimated time: 15 minutes
       Downtime required: Yes
       
       Proceed? (yes/no)

You: yes

clide: ✓ Exporting MySQL data (2.1 GB)
       ✓ Converting schema
       ✓ Creating PostgreSQL tables
       ✓ Importing data
       ✓ Creating indexes
       ✓ Verifying data (100% match)
       
       Migration summary:
       - Tables: 47
       - Rows: 1,234,567
       - Duration: 12 minutes
       
       🛬 Migration complete!
```

---

## 📊 Monitoring & Alerts

### Setup Monitoring

```
You: "Monitor my VPS and alert me if issues"

clide: 🛫 Setting up monitoring...
       
       ✓ Netdata installed
       ✓ Prometheus exporters configured
       ✓ Grafana dashboards imported
       
       Alert rules configured:
       - CPU >80% for 5 minutes
       - Memory >90% for 2 minutes
       - Disk >85%
       - Failed SSH attempts >5
       - Service downtime
       - SSL expiry <30 days
       
       Notification channels:
       - Signal (high priority)
       - Email (daily digest)
       
       Dashboard: http://yourip:19999
       
       🛬 Monitoring active!
```

### Proactive Health Checks

```
[15 minutes later - clide sends you a message]

clide: ⚠️  Alert: Disk usage high
       /dev/sda1: 87% (54GB/62GB)
       
       Top consumers:
       - /var/log: 18GB (+6GB since last check)
       - /tmp: 12GB (+4GB)
       
       Recommended actions:
       1. Clean old logs (would free ~15GB)
       2. Clear tmp files (would free ~10GB)
       
       Execute cleanup? (yes/no)
```

---

## 🔄 Automated Workflows

### Scheduled Tasks

```
You: "Run database backup daily at 2 AM"

clide: 🛫 Scheduling backup...
       ✓ Cron job created
       ✓ Schedule: 0 2 * * *
       ✓ Backup script: /opt/clide/backup.sh
       ✓ Logs: /var/log/backups.log
       ✓ Notifications: Signal on failure
       
       First run: Tomorrow at 2:00 AM
       🛬 Done!
```

### Conditional Actions

```
You: "If disk usage >90%, clean logs automatically"

clide: 🛫 Creating conditional action...
       ✓ Monitor created
       ✓ Check interval: 15 minutes
       ✓ Trigger: disk >90%
       ✓ Action: clean old logs (>30 days)
       ✓ Safety: dry-run first
       ✓ Notification: before and after
       
       🛬 Auto-cleanup active!
```

---

## 🔧 Troubleshooting Workflows

### Debug Application

```
You: "My app is slow, find the issue"

clide: 🛫 Investigating performance...
       
       ✓ Checking CPU usage... OK (15%)
       ✓ Checking memory... OK (45%)
       ✓ Checking disk I/O... ⚠️  HIGH
       
       Found issue: Database queries slow
       
       ✓ Analyzing PostgreSQL
       ✓ Found: Missing index on users.email
       ✓ Query time: 2.5s → 0.03s (with index)
       
       Fix available:
       CREATE INDEX idx_users_email ON users(email);
       
       Apply fix? (yes/no)

You: yes

clide: ✓ Index created
       ✓ Query performance improved 83x
       ✓ App response time: 2.8s → 0.2s
       
       🛬 Issue resolved!
```

### Service Recovery

```
You: "Nginx crashed, fix it"

clide: 🛫 Investigating nginx crash...
       
       ✓ Analyzing crash logs
       Found: Out of memory (OOM killer)
       Root cause: Memory leak in config
       
       Recovery plan:
       1. Restart nginx ✓
       2. Fix config ✓
       3. Increase swap ✓
       4. Add memory monitoring ✓
       
       Preventive measures:
       ✓ PM2 process manager added
       ✓ Auto-restart on crash
       ✓ Memory alerts (>80%)
       
       🛬 Service restored!
       Uptime: 99.97%
```

---

## 📦 Bulk Operations

### Multi-VPS Management

```
You: "Update all VPS servers"

clide: 🛫 Updating 3 servers...
       
       [Production VPS]
       ✓ apt update && upgrade (47 packages)
       ✓ Reboot required: No
       
       [Staging VPS]
       ✓ apt update && upgrade (52 packages)
       ✓ Reboot required: No
       
       [Dev VPS]
       ✓ apt update && upgrade (39 packages)
       ✓ Reboot required: Yes
       
       Schedule dev reboot? (yes/no)
```

### Batch File Operations

```
You: "Convert all .jpg images to .webp in uploads folder"

clide: 🛫 Converting images...
       
       Found: 1,247 images (2.3 GB)
       
       [DRY-RUN] Would convert:
       - uploads/photo1.jpg → photo1.webp
       - uploads/photo2.jpg → photo2.webp
       - ... (1,245 more)
       
       Estimated size: 2.3 GB → 890 MB (61% reduction)
       Estimated time: 8 minutes
       
       Proceed? (yes/no)

You: yes

clide: ✓ Converting... [=========>   ] 75% (937/1247)
       ✓ Complete! 
       ✓ Saved: 1.41 GB
       ✓ Original backups: /uploads/backup/
       
       🛬 Done!
```

---

## 🎯 Advanced Workflows

### CI/CD Pipeline

```
You: "Setup CI/CD for my GitHub repo"

clide: 🛫 Setting up CI/CD...
       
       ✓ GitHub webhook configured
       ✓ Deploy script created
       ✓ Systemd service created
       
       Workflow:
       1. Push to main branch
       2. Webhook triggers deploy
       3. Pull latest code
       4. Run tests
       5. Build application
       6. Zero-downtime restart
       7. Notify on Signal
       
       Test workflow? (yes/no)
```

### Blue-Green Deployment

```
You: "Deploy new version with zero downtime"

clide: 🛫 Blue-green deployment...
       
       Current (blue): v1.2.3
       New (green): v1.3.0
       
       ✓ Green environment created
       ✓ Application deployed to green
       ✓ Running health checks...
       ✓ All checks passed
       
       Switch traffic to green? (yes/no)

You: yes

clide: ✓ Nginx updated (blue → green)
       ✓ Traffic switched (0 downtime)
       ✓ Monitoring green for 5 minutes...
       ✓ Green stable
       ✓ Blue environment stopped
       
       Deployment complete!
       Rollback available: yes
       
       🛬 Version 1.3.0 live!
```

---

## 💡 Tips & Tricks

### Natural Language Flexibility

```
# All of these work:
"clean up logs"
"remove old logs"
"delete logs older than 30 days"
"my logs are taking up space, fix it"
```

### Context Awareness

```
You: "setup postgres container"
clide: [creates container]

You: "now deploy my app using it"
clide: [remembers postgres details, configures app]

You: "backup both"
clide: [backs up postgres AND app data]
```

### Multi-Step Operations

```
You: "I want to migrate from Apache to Nginx"

clide: This will take multiple steps:
       1. Install Nginx
       2. Convert Apache config
       3. Test Nginx
       4. Switch traffic
       5. Stop Apache
       
       Start step 1? (yes/no/show-details)
```

---

## 📞 Need Help?

- 💬 [Discussions](https://github.com/yourusername/clide/discussions)
- 📖 [Full Documentation](../README.md)
- 🐛 [Report Issues](https://github.com/yourusername/clide/issues)

---

**More workflow examples? Contribute yours!** See [CONTRIBUTING.md](../CONTRIBUTING.md)

**Happy gliding!** ✈️

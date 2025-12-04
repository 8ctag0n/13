FROM docker.io/library/nginx:alpine

# Copy nginx config
COPY nginx.conf /etc/nginx/nginx.conf

# Copy IDL with correct permissions
COPY src/programs/zyberlink/idl/zyberlink.json /etc/nginx/idl.json
RUN chmod 644 /etc/nginx/idl.json

EXPOSE 80

CMD ["nginx", "-g", "daemon off;"]
